use std::ffi::c_int;

use crate::{log_noop, EOF};

#[derive(Debug)]
pub enum EncodeError {
    EndOfBuffer,
    ZeroSizeInput,
}

#[derive(Debug, Default)]
struct Buffer {
    bitbuffer: c_int,  /* bit buffer			*/
    bits_to_go: c_int, /* bits to go in buffer	*/
    current: usize,    /* current position in buffer	*/
}

pub struct RCEncoder {
    log_fn: fn(&str),
    buffer: Buffer,
}

impl Default for RCEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl RCEncoder {
    pub fn new() -> Self {
        RCEncoder {
            log_fn: log_noop,
            buffer: Buffer::default(),
        }
    }

    pub fn set_log_fn(&mut self, log_fn: fn(&str)) {
        self.log_fn = log_fn;
    }

    pub fn encode(
        &mut self,
        input: &[i32],        /* input array			*/
        nx: usize,            /* number of input pixels	*/
        nblock: usize,        /* coding block size		*/
        output: &mut Vec<u8>, /* output buffer		*/
    ) -> Result<usize, EncodeError> {
        if input.is_empty() || nblock == 0 {
            return Err(EncodeError::ZeroSizeInput);
        }

        let mut nextpix: i32;
        let mut pdiff: i32;

        let mut v: i32;
        let mut fs: i32;
        let mut fsmask: i32;
        let mut top: i32;

        let mut lbitbuffer: i32;
        let mut lbits_to_go: i32;

        let mut psum: u32;
        let mut pixelsum: f64;
        let mut dpsum: f64;

        /*
         * From bsize derive:
         * FSBITS = # bits required to store FS
         * FSMAX = maximum value for FS
         * BBITS = bits/pixel for direct coding
         */

        /* move out of switch block, to tweak performance */
        let fsbits: i32 = 5;
        let fsmax: i32 = 25;

        let bbits: i32 = 1 << fsbits;

        /*
         * Set up buffer pointers
         */
        self.buffer = Buffer {
            current: 0,
            bits_to_go: 8,
            bitbuffer: 0,
        };

        output.reserve(nx * 4);

        /*
         * array for differences mapped to non-negative values
         */
        let mut diff: Vec<u32> = vec![0; nblock];

        /*
         * Code in blocks of nblock pixels
         */

        // Initialize for bit output
        self.buffer.bitbuffer = 0;
        self.buffer.bits_to_go = 8;

        /* write out first int value to the first 4 bytes of the buffer */
        if self.output_nbits(output, input[0], 32) == EOF {
            (self.log_fn)("rice_encode: end of buffer");
            return Err(EncodeError::EndOfBuffer);
        }

        let mut lastpix: i32 = input[0]; /* the first difference will always be zero */

        let mut thisblock: usize = nblock;

        for i in (0..nx).step_by(nblock) {
            // for (i=0; i<nx; i += nblock) {
            /* last block may be shorter */
            if nx - i < nblock {
                thisblock = nx - i;
            }
            /*
             * Compute differences of adjacent pixels and map them to unsigned values.
             * Note that this may overflow the integer variables -- that's
             * OK, because we can recover when decompressing.  If we were
             * compressing shorts or bytes, would want to do this arithmetic
             * with short/byte working variables (though diff will still be
             * passed as an int.)
             *
             * compute sum of mapped pixel values at same time
             * use double precision for sum to allow 32-bit integer inputs
             */
            pixelsum = 0.0;
            for j in 0..thisblock {
                nextpix = input[i + j];
                pdiff = nextpix.wrapping_sub(lastpix);
                diff[j] = (if pdiff < 0 { !(pdiff << 1) } else { pdiff << 1 }) as u32; // ! is bitwise complement
                pixelsum += diff[j] as f64;
                lastpix = nextpix;
            }

            /*
             * compute number of bits to split from sum
             */
            dpsum = (pixelsum - ((thisblock as f64) / 2.0) - 1.0) / (thisblock as f64);
            if dpsum < 0.0 {
                dpsum = 0.0;
            }
            psum = (dpsum as u32) >> 1;

            fs = 0;
            while psum > 0 {
                psum >>= 1;
                fs += 1;
            }

            /*
             * write the codes
             * fsbits ID bits used to indicate split level
             */
            if fs >= fsmax {
                /* Special high entropy case when FS >= fsmax
                 * Just write pixel difference values directly, no Rice coding at all.
                 */
                if self.output_nbits(output, fsmax + 1, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }

                for &diff_item in diff.iter().take(thisblock) {
                    if self.output_nbits(output, diff_item as i32, bbits) == EOF {
                        (self.log_fn)("rice_encode: end of buffer");
                        return Err(EncodeError::EndOfBuffer);
                    }
                }
            } else if fs == 0 && pixelsum == 0.0 {
                /*
                 * special low entropy case when FS = 0 and pixelsum=0 (all
                 * pixels in block are zero.)
                 * Output a 0 and return
                 */
                if self.output_nbits(output, 0, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }
            } else {
                /* normal case: not either very high or very low entropy */
                if self.output_nbits(output, fs + 1, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }
                fsmask = (1 << fs) - 1;
                /*
                 * local copies of bit buffer to improve optimization
                 */
                lbitbuffer = self.buffer.bitbuffer;
                lbits_to_go = self.buffer.bits_to_go;
                for &diff_item in diff.iter().take(thisblock) {
                    v = diff_item as i32;
                    top = v >> fs;
                    /*
                     * top is coded by top zeros + 1
                     */
                    if lbits_to_go > top {
                        lbitbuffer = lbitbuffer.wrapping_shl((top + 1) as u32);
                        lbitbuffer |= 1;
                        lbits_to_go -= top + 1;
                    } else {
                        lbitbuffer <<= lbits_to_go;
                        self.putcbuf(output, lbitbuffer & 0xff);

                        top -= lbits_to_go;
                        while top >= 8 {
                            self.putcbuf(output, 0);
                            top -= 8;
                        }

                        lbitbuffer = 1;
                        lbits_to_go = 7 - top;
                    }
                    /*
                     * bottom FS bits are written without coding
                     * code is output_nbits, moved into this routine to reduce overheads
                     * This code potentially breaks if FS>24, so I am limiting
                     * FS to 24 by choice of FSMAX above.
                     */
                    if fs > 0 {
                        lbitbuffer <<= fs;
                        lbitbuffer |= v & fsmask;
                        lbits_to_go -= fs;
                        while lbits_to_go <= 0 {
                            self.putcbuf(output, (lbitbuffer >> (-lbits_to_go)) & 0xff);
                            lbits_to_go += 8;
                        }
                    }
                }

                self.buffer.bitbuffer = lbitbuffer;
                self.buffer.bits_to_go = lbits_to_go;
            }
        }

        // Flush out the last bits
        if self.buffer.bits_to_go < 8 {
            self.putcbuf(output, self.buffer.bitbuffer << self.buffer.bits_to_go);
        }

        // return number of bytes used
        Ok(self.buffer.current)
    }

    pub fn encode_short(
        &mut self,
        input: &[i16],        /* input array			*/
        nx: usize,            /* number of input pixels	*/
        nblock: usize,        /* coding block size		*/
        output: &mut Vec<u8>, /* output buffer		*/
    ) -> Result<usize, EncodeError> {
        if input.is_empty() || nblock == 0 {
            return Err(EncodeError::ZeroSizeInput);
        }

        let mut nextpix: i16;
        let mut pdiff: i16;

        let mut v: i32;
        let mut fs: i32;
        let mut fsmask: i32;
        let mut top: i32;

        let mut lbitbuffer: i32;
        let mut lbits_to_go: i32;

        let mut psum: u16;
        let mut pixelsum: f64;
        let mut dpsum: f64;

        /*
         * From bsize derive:
         * FSBITS = # bits required to store FS
         * FSMAX = maximum value for FS
         * BBITS = bits/pixel for direct coding
         */

        /* move out of switch block, to tweak performance */
        let fsbits: i32 = 4;
        let fsmax: i32 = 14;

        let bbits: i32 = 1 << fsbits;

        /*
         * Set up buffer pointers
         */
        self.buffer = Buffer {
            current: 0,
            bits_to_go: 8,
            bitbuffer: 0,
        };

        output.reserve(nx * 4);

        /*
         * array for differences mapped to non-negative values
         */
        let mut diff: Vec<u32> = vec![0; nblock];

        /*
         * Code in blocks of nblock pixels
         */

        // Initialize for bit output
        self.buffer.bitbuffer = 0;
        self.buffer.bits_to_go = 8;

        /* write out first int value to the first 4 bytes of the buffer */
        if self.output_nbits(output, input[0].into(), 16) == EOF {
            (self.log_fn)("rice_encode: end of buffer");
            return Err(EncodeError::EndOfBuffer);
        }

        let mut lastpix: i16 = input[0]; /* the first difference will always be zero */

        let mut thisblock: usize = nblock;

        for i in (0..nx).step_by(nblock) {
            // for (i=0; i<nx; i += nblock) {
            /* last block may be shorter */
            if nx - i < nblock {
                thisblock = nx - i;
            }
            /*
             * Compute differences of adjacent pixels and map them to unsigned values.
             * Note that this may overflow the integer variables -- that's
             * OK, because we can recover when decompressing.  If we were
             * compressing shorts or bytes, would want to do this arithmetic
             * with short/byte working variables (though diff will still be
             * passed as an int.)
             *
             * compute sum of mapped pixel values at same time
             * use double precision for sum to allow 32-bit integer inputs
             */
            pixelsum = 0.0;
            for j in 0..thisblock {
                nextpix = input[i + j];
                pdiff = nextpix.wrapping_sub(lastpix);
                diff[j] = (if pdiff < 0 { !(pdiff << 1) } else { pdiff << 1 }) as u32; // ! is bitwise complement
                pixelsum += diff[j] as f64;
                lastpix = nextpix;
            }

            /*
             * compute number of bits to split from sum
             */
            dpsum = (pixelsum - ((thisblock as f64) / 2.0) - 1.0) / (thisblock as f64);
            if dpsum < 0.0 {
                dpsum = 0.0;
            }
            psum = (dpsum as u16) >> 1;

            fs = 0;
            while psum > 0 {
                psum >>= 1;
                fs += 1;
            }

            /*
             * write the codes
             * fsbits ID bits used to indicate split level
             */
            if fs >= fsmax {
                /* Special high entropy case when FS >= fsmax
                 * Just write pixel difference values directly, no Rice coding at all.
                 */
                if self.output_nbits(output, fsmax + 1, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }

                for &diff_item in diff.iter().take(thisblock) {
                    if self.output_nbits(output, diff_item as i32, bbits) == EOF {
                        (self.log_fn)("rice_encode: end of buffer");
                        return Err(EncodeError::EndOfBuffer);
                    }
                }
            } else if fs == 0 && pixelsum == 0.0 {
                /*
                 * special low entropy case when FS = 0 and pixelsum=0 (all
                 * pixels in block are zero.)
                 * Output a 0 and return
                 */
                if self.output_nbits(output, 0, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }
            } else {
                /* normal case: not either very high or very low entropy */
                if self.output_nbits(output, fs + 1, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }
                fsmask = (1 << fs) - 1;
                /*
                 * local copies of bit buffer to improve optimization
                 */
                lbitbuffer = self.buffer.bitbuffer;
                lbits_to_go = self.buffer.bits_to_go;
                for &diff_item in diff.iter().take(thisblock) {
                    v = diff_item as i32;
                    top = v >> fs;
                    /*
                     * top is coded by top zeros + 1
                     */
                    if lbits_to_go > top {
                        lbitbuffer = lbitbuffer.wrapping_shl((top + 1) as u32);
                        lbitbuffer |= 1;
                        lbits_to_go -= top + 1;
                    } else {
                        lbitbuffer <<= lbits_to_go;
                        self.putcbuf(output, lbitbuffer & 0xff);

                        top -= lbits_to_go;
                        while top >= 8 {
                            self.putcbuf(output, 0);
                            top -= 8;
                        }

                        lbitbuffer = 1;
                        lbits_to_go = 7 - top;
                    }
                    /*
                     * bottom FS bits are written without coding
                     * code is output_nbits, moved into this routine to reduce overheads
                     * This code potentially breaks if FS>24, so I am limiting
                     * FS to 24 by choice of FSMAX above.
                     */
                    if fs > 0 {
                        lbitbuffer <<= fs;
                        lbitbuffer |= v & fsmask;
                        lbits_to_go -= fs;
                        while lbits_to_go <= 0 {
                            self.putcbuf(output, (lbitbuffer >> (-lbits_to_go)) & 0xff);
                            lbits_to_go += 8;
                        }
                    }
                }

                self.buffer.bitbuffer = lbitbuffer;
                self.buffer.bits_to_go = lbits_to_go;
            }
        }

        // Flush out the last bits
        if self.buffer.bits_to_go < 8 {
            self.putcbuf(output, self.buffer.bitbuffer << self.buffer.bits_to_go);
        }

        // return number of bytes used
        Ok(self.buffer.current)
    }

    pub fn encode_byte(
        &mut self,
        input: &[i8],         /* input array			*/
        nx: usize,            /* number of input pixels	*/
        nblock: usize,        /* coding block size		*/
        output: &mut Vec<u8>, /* output buffer		*/
    ) -> Result<usize, EncodeError> {
        if input.is_empty() || nblock == 0 {
            return Err(EncodeError::ZeroSizeInput);
        }

        let mut nextpix: i8;
        let mut pdiff: i8;

        let mut v: i32;
        let mut fs: i32;
        let mut fsmask: i32;
        let mut top: i32;

        let mut lbitbuffer: i32;
        let mut lbits_to_go: i32;

        let mut psum: u8;
        let mut pixelsum: f64;
        let mut dpsum: f64;

        /*
         * From bsize derive:
         * FSBITS = # bits required to store FS
         * FSMAX = maximum value for FS
         * BBITS = bits/pixel for direct coding
         */

        /* move out of switch block, to tweak performance */
        let fsbits: i32 = 3;
        let fsmax: i32 = 6;

        let bbits: i32 = 1 << fsbits;

        /*
         * Set up buffer pointers
         */
        self.buffer = Buffer {
            current: 0,
            bits_to_go: 8,
            bitbuffer: 0,
        };

        output.reserve(nx * 4);

        /*
         * array for differences mapped to non-negative values
         */
        let mut diff: Vec<u32> = vec![0; nblock];

        /*
         * Code in blocks of nblock pixels
         */

        // Initialize for bit output
        self.buffer.bitbuffer = 0;
        self.buffer.bits_to_go = 8;

        /* write out first int value to the first 4 bytes of the buffer */
        if self.output_nbits(output, input[0].into(), 8) == EOF {
            (self.log_fn)("rice_encode: end of buffer");
            return Err(EncodeError::EndOfBuffer);
        }

        let mut lastpix: i8 = input[0]; /* the first difference will always be zero */

        let mut thisblock: usize = nblock;

        for i in (0..nx).step_by(nblock) {
            // for (i=0; i<nx; i += nblock) {
            /* last block may be shorter */
            if nx - i < nblock {
                thisblock = nx - i;
            }
            /*
             * Compute differences of adjacent pixels and map them to unsigned values.
             * Note that this may overflow the integer variables -- that's
             * OK, because we can recover when decompressing.  If we were
             * compressing shorts or bytes, would want to do this arithmetic
             * with short/byte working variables (though diff will still be
             * passed as an int.)
             *
             * compute sum of mapped pixel values at same time
             * use double precision for sum to allow 32-bit integer inputs
             */
            pixelsum = 0.0;
            for j in 0..thisblock {
                nextpix = input[i + j];
                pdiff = nextpix.wrapping_sub(lastpix);
                diff[j] = (if pdiff < 0 { !(pdiff << 1) } else { pdiff << 1 }) as u32; // ! is bitwise complement
                pixelsum += diff[j] as f64;
                lastpix = nextpix;
            }

            /*
             * compute number of bits to split from sum
             */
            dpsum = (pixelsum - ((thisblock as f64) / 2.0) - 1.0) / (thisblock as f64);
            if dpsum < 0.0 {
                dpsum = 0.0;
            }
            psum = (dpsum as u8) >> 1;

            fs = 0;
            while psum > 0 {
                psum >>= 1;
                fs += 1;
            }

            /*
             * write the codes
             * fsbits ID bits used to indicate split level
             */
            if fs >= fsmax {
                /* Special high entropy case when FS >= fsmax
                 * Just write pixel difference values directly, no Rice coding at all.
                 */
                if self.output_nbits(output, fsmax + 1, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }

                for &diff_item in diff.iter().take(thisblock) {
                    if self.output_nbits(output, diff_item as i32, bbits) == EOF {
                        (self.log_fn)("rice_encode: end of buffer");
                        return Err(EncodeError::EndOfBuffer);
                    }
                }
            } else if fs == 0 && pixelsum == 0.0 {
                /*
                 * special low entropy case when FS = 0 and pixelsum=0 (all
                 * pixels in block are zero.)
                 * Output a 0 and return
                 */
                if self.output_nbits(output, 0, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }
            } else {
                /* normal case: not either very high or very low entropy */
                if self.output_nbits(output, fs + 1, fsbits) == EOF {
                    (self.log_fn)("rice_encode: end of buffer");
                    return Err(EncodeError::EndOfBuffer);
                }
                fsmask = (1 << fs) - 1;
                /*
                 * local copies of bit buffer to improve optimization
                 */
                lbitbuffer = self.buffer.bitbuffer;
                lbits_to_go = self.buffer.bits_to_go;
                for &diff_item in diff.iter().take(thisblock) {
                    v = diff_item as i32;
                    top = v >> fs;
                    /*
                     * top is coded by top zeros + 1
                     */
                    if lbits_to_go > top {
                        lbitbuffer = lbitbuffer.wrapping_shl((top + 1) as u32);
                        lbitbuffer |= 1;
                        lbits_to_go -= top + 1;
                    } else {
                        lbitbuffer <<= lbits_to_go;
                        self.putcbuf(output, lbitbuffer & 0xff);

                        top -= lbits_to_go;
                        while top >= 8 {
                            self.putcbuf(output, 0);
                            top -= 8;
                        }

                        lbitbuffer = 1;
                        lbits_to_go = 7 - top;
                    }
                    /*
                     * bottom FS bits are written without coding
                     * code is output_nbits, moved into this routine to reduce overheads
                     * This code potentially breaks if FS>24, so I am limiting
                     * FS to 24 by choice of FSMAX above.
                     */
                    if fs > 0 {
                        lbitbuffer <<= fs;
                        lbitbuffer |= v & fsmask;
                        lbits_to_go -= fs;
                        while lbits_to_go <= 0 {
                            self.putcbuf(output, (lbitbuffer >> (-lbits_to_go)) & 0xff);
                            lbits_to_go += 8;
                        }
                    }
                }

                self.buffer.bitbuffer = lbitbuffer;
                self.buffer.bits_to_go = lbits_to_go;
            }
        }

        // Flush out the last bits
        if self.buffer.bits_to_go < 8 {
            self.putcbuf(output, self.buffer.bitbuffer << self.buffer.bits_to_go);
        }

        // return number of bytes used
        Ok(self.buffer.current)
    }

    /*---------------------------------------------------------------------------*/
    /// Output N bits (N must be <= 32)
    fn output_nbits(&mut self, output: &mut Vec<u8>, bits: i32, n: i32) -> i32 {
        /* local copies */

        let mut n = n;

        /* AND mask for the right-most n bits */
        static MASK: [u32; 33] = [
            0, 0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff,
            0x3fff, 0x7fff, 0xffff, 0x1ffff, 0x3ffff, 0x7ffff, 0xfffff, 0x1fffff, 0x3fffff,
            0x7fffff, 0xffffff, 0x1ffffff, 0x3ffffff, 0x7ffffff, 0xfffffff, 0x1fffffff, 0x3fffffff,
            0x7fffffff, 0xffffffff,
        ];

        /*
         * insert bits at end of bitbuffer
         */
        let mut lbitbuffer: i32 = self.buffer.bitbuffer;
        let mut lbits_to_go: i32 = self.buffer.bits_to_go;
        if lbits_to_go + n > 32 {
            /*
             * special case for large n: put out the top lbits_to_go bits first
             * note that 0 < lbits_to_go <= 8
             */
            lbitbuffer <<= lbits_to_go;
            /*	lbitbuffer |= (bits>>(n-lbits_to_go)) & ((1<<lbits_to_go)-1); */
            lbitbuffer |= (bits >> (n - lbits_to_go)) & (MASK[lbits_to_go as usize] as i32);
            self.putcbuf(output, lbitbuffer & 0xff);
            n -= lbits_to_go;
            lbits_to_go = 8;
        }
        lbitbuffer <<= n;
        /*    lbitbuffer |= ( bits & ((1<<n)-1) ); */
        lbitbuffer |= bits & MASK[n as usize] as i32;
        lbits_to_go -= n;
        while lbits_to_go <= 0 {
            /*
             * bitbuffer full, put out top 8 bits
             */
            self.putcbuf(output, (lbitbuffer >> (-lbits_to_go)) & 0xff);
            lbits_to_go += 8;
        }
        self.buffer.bitbuffer = lbitbuffer;
        self.buffer.bits_to_go = lbits_to_go;
        0
    }

    fn putcbuf(&mut self, buffer: &mut Vec<u8>, c: i32) {
        buffer.push(c as u8);
        self.buffer.current += 1;
    }
}
