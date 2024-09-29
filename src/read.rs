use std::ffi::{c_uchar, c_uint, c_ushort};

use crate::log_noop;

/// nonzero_count is lookup table giving number of bits in 8-bit values not including
/// leading zeros used in fits_rdecomp, fits_rdecomp_short and fits_rdecomp_byte
const NONZERO_COUNT: [i32; 256] = [
    0, 1, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
];

#[derive(Debug)]
pub enum DecodeError {
    EndOfBuffer,
    ZeroSizeInput,
}

pub struct RCDecoder {
    log_fn: fn(&str),
}

impl Default for RCDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl RCDecoder {
    pub fn new() -> RCDecoder {
        RCDecoder { log_fn: log_noop }
    }

    pub fn set_log_fn(&mut self, log_fn: fn(&str)) {
        self.log_fn = log_fn;
    }

    pub fn decode(
        &self,
        input: &[u8], /* input buffer			*/
        nx: usize,    /* number of output pixels	*/
        nblock: usize,
        output: &mut Vec<c_uint>,
    ) -> Result<(), DecodeError> /* coding block size		*/ {
        /* int bsize;  */

        let mut k: i32;
        let mut imax: usize;

        let mut nzero: i32;
        let mut fs: i32;

        let mut diff: u32;

        output.resize(nx, 0);
        output.fill(0);

        /*
         * Original size of each pixel (bsize, bytes) and coding block
         * size (nblock, pixels)
         * Could make bsize a parameter to allow more efficient
         * compression of short & byte images.
         */
        /*    bsize = 4; */

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
         * Decode in blocks of nblock pixels
         */

        /* first 4 bytes of input buffer contain the value of the first */
        /* 4 byte integer value, without any encoding */

        let mut lastpix: u32 = 0;
        let mut bytevalue: u8 = input[0];
        lastpix |= (bytevalue as u32) << 24;
        bytevalue = input[1];
        lastpix |= (bytevalue as u32) << 16;
        bytevalue = input[2];
        lastpix |= (bytevalue as u32) << 8;
        bytevalue = input[3];
        lastpix |= bytevalue as u32;

        let mut c_current: usize = 4;

        // cend = c + clen - 4;

        let mut b: u32 = input[c_current] as u32; /* bit buffer			*/
        c_current += 1;
        let mut nbits: i32 = 8; /* number of bits remaining in b	*/

        let mut i: usize = 0;
        while i < nx {
            /* get the FS value from first fsbits */
            nbits -= fsbits;
            while nbits < 0 {
                b = (b << 8) | input[c_current] as u32;
                c_current += 1;
                nbits += 8;
            }
            fs = ((b >> nbits).wrapping_sub(1)) as i32;

            b &= (1 << nbits) - 1;
            /* loop over the next block */
            imax = i + nblock;
            if imax > nx {
                imax = nx;
            }
            if fs < 0 {
                /* low-entropy case, all zero differences */
                while i < imax {
                    output[i] = lastpix;
                    i += 1;
                }
            } else if fs == fsmax {
                /* high-entropy case, directly coded pixel values */
                while i < imax {
                    k = bbits - nbits;
                    diff = b.wrapping_shl(k as u32);
                    k -= 8;
                    while k >= 0 {
                        b = input[c_current] as u32;
                        c_current += 1;
                        diff |= b << k;
                        k -= 8
                    }
                    if nbits > 0 {
                        b = input[c_current] as u32;
                        c_current += 1;
                        diff |= b >> (-k);
                        b &= (1 << nbits) - 1;
                    } else {
                        b = 0;
                    }
                    /*
                     * undo mapping and differencing
                     * Note that some of these operations will overflow the
                     * unsigned int arithmetic -- that's OK, it all works
                     * out to give the right answers in the output file.
                     */
                    if (diff & 1) == 0 {
                        diff >>= 1;
                    } else {
                        diff = !(diff >> 1);
                    }
                    output[i] = diff.wrapping_add(lastpix);
                    lastpix = output[i];
                    i += 1;
                }
            } else {
                /* normal case, Rice coding */
                while i < imax {
                    /* count number of leading zeros */
                    while b == 0 {
                        nbits += 8;

                        b = input[c_current] as u32;
                        c_current += 1;
                    }
                    nzero = nbits - NONZERO_COUNT[b as usize];
                    nbits -= nzero + 1;
                    /* flip the leading one-bit */
                    b ^= 1 << nbits;
                    /* get the FS trailing bits */
                    nbits -= fs;
                    while nbits < 0 {
                        b = (b << 8) | (input[c_current] as u32);

                        c_current += 1;
                        nbits += 8;
                    }
                    diff = ((nzero as u32) << fs) | (b >> nbits);
                    b &= (1 << nbits) - 1;

                    /* undo mapping and differencing */
                    if (diff & 1) == 0 {
                        diff >>= 1;
                    } else {
                        diff = !(diff >> 1);
                    }
                    output[i] = diff.wrapping_add(lastpix);
                    lastpix = output[i];
                    i += 1;
                }
            }
            if c_current > input.len() {
                (self.log_fn)("decompression error: hit end of compressed byte stream");
                return Err(DecodeError::EndOfBuffer);
            }
        }
        if c_current < input.len() {
            (self.log_fn)("decompression warning: unused bytes at end of compressed buffer");
        }

        Ok(())
    }

    pub fn decode_short(
        &self,
        input: &[u8], /* input buffer			*/
        nx: usize,    /* number of output pixels	*/
        nblock: usize,
        output: &mut Vec<c_ushort>,
    ) -> Result<(), DecodeError> /* coding block size		*/ {
        /* int bsize;  */

        let mut k: i32;
        let mut imax: usize;

        let mut nzero: i32;
        let mut fs: i32;

        let mut diff: u32;

        output.resize(nx, 0);
        output.fill(0);

        /*
         * Original size of each pixel (bsize, bytes) and coding block
         * size (nblock, pixels)
         * Could make bsize a parameter to allow more efficient
         * compression of short & byte images.
         */
        /*    bsize = 2; */

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
         * Decode in blocks of nblock pixels
         */

        /* first 2 bytes of input buffer contain the value of the first */
        /* 2 byte integer value, without any encoding */

        let mut lastpix: u32 = 0;
        let mut bytevalue: u8 = input[0];
        lastpix |= (bytevalue as u32) << 8;
        bytevalue = input[1];
        lastpix |= bytevalue as u32;

        let mut c_current: usize = 2;

        // cend = c + clen - 2;

        let mut b: u32 = input[c_current] as u32; /* bit buffer			*/
        c_current += 1;
        let mut nbits: i32 = 8; /* number of bits remaining in b	*/

        let mut i: usize = 0;
        while i < nx {
            /* get the FS value from first fsbits */
            nbits -= fsbits;
            while nbits < 0 {
                b = (b << 8) | input[c_current] as u32;
                c_current += 1;
                nbits += 8;
            }
            fs = ((b >> nbits).wrapping_sub(1)) as i32;

            b &= (1 << nbits) - 1;
            /* loop over the next block */
            imax = i + nblock;
            if imax > nx {
                imax = nx;
            }
            if fs < 0 {
                /* low-entropy case, all zero differences */
                while i < imax {
                    output[i] = lastpix as c_ushort;
                    i += 1;
                }
            } else if fs == fsmax {
                /* high-entropy case, directly coded pixel values */
                while i < imax {
                    k = bbits - nbits;
                    diff = b.wrapping_shl(k as u32);
                    k -= 8;
                    while k >= 0 {
                        b = input[c_current] as u32;
                        c_current += 1;
                        diff |= b << k;
                        k -= 8
                    }
                    if nbits > 0 {
                        b = input[c_current] as u32;
                        c_current += 1;
                        diff |= b >> (-k);
                        b &= (1 << nbits) - 1;
                    } else {
                        b = 0;
                    }
                    /*
                     * undo mapping and differencing
                     * Note that some of these operations will overflow the
                     * unsigned int arithmetic -- that's OK, it all works
                     * out to give the right answers in the output file.
                     */
                    if (diff & 1) == 0 {
                        diff >>= 1;
                    } else {
                        diff = !(diff >> 1);
                    }
                    output[i] = diff.wrapping_add(lastpix) as c_ushort;
                    lastpix = output[i] as u32;
                    i += 1;
                }
            } else {
                /* normal case, Rice coding */
                while i < imax {
                    /* count number of leading zeros */
                    while b == 0 {
                        nbits += 8;

                        b = input[c_current] as u32;
                        c_current += 1;
                    }
                    nzero = nbits - NONZERO_COUNT[b as usize];
                    nbits -= nzero + 1;
                    /* flip the leading one-bit */
                    b ^= 1 << nbits;
                    /* get the FS trailing bits */
                    nbits -= fs;
                    while nbits < 0 {
                        b = (b << 8) | (input[c_current] as u32);

                        c_current += 1;
                        nbits += 8;
                    }
                    diff = ((nzero as u32) << fs) | (b >> nbits);
                    b &= (1 << nbits) - 1;

                    /* undo mapping and differencing */
                    if (diff & 1) == 0 {
                        diff >>= 1;
                    } else {
                        diff = !(diff >> 1);
                    }
                    output[i] = diff.wrapping_add(lastpix) as c_ushort;
                    lastpix = output[i] as u32;
                    i += 1;
                }
            }
            if c_current > input.len() {
                (self.log_fn)("decompression error: hit end of compressed byte stream");
                return Err(DecodeError::EndOfBuffer);
            }
        }
        if c_current < input.len() {
            (self.log_fn)("decompression warning: unused bytes at end of compressed buffer");
        }

        Ok(())
    }

    pub fn decode_byte(
        &self,
        input: &[u8], /* input buffer			*/
        nx: usize,    /* number of output pixels	*/
        nblock: usize,
        output: &mut Vec<c_uchar>,
    ) -> Result<(), DecodeError> /* coding block size		*/ {
        /* int bsize;  */

        let mut k: i32;
        let mut imax: usize;

        let mut nzero: i32;
        let mut fs: i32;

        let mut diff: u32;

        output.resize(nx, 0);
        output.fill(0);

        /*
         * Original size of each pixel (bsize, bytes) and coding block
         * size (nblock, pixels)
         * Could make bsize a parameter to allow more efficient
         * compression of short & byte images.
         */
        /*    bsize = 1; */

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
         * Decode in blocks of nblock pixels
         */

        /* first byte of input buffer contain the value of the first */
        /* byte integer value, without any encoding */

        let mut lastpix: u32 = input[0] as u32;

        let mut c_current: usize = 1;

        // cend = c + clen - 2;

        let mut b: u32 = input[c_current] as u32; /* bit buffer			*/
        c_current += 1;
        let mut nbits: i32 = 8; /* number of bits remaining in b	*/

        let mut i: usize = 0;
        while i < nx {
            /* get the FS value from first fsbits */
            nbits -= fsbits;
            while nbits < 0 {
                b = (b << 8) | input[c_current] as u32;
                c_current += 1;
                nbits += 8;
            }
            fs = ((b >> nbits).wrapping_sub(1)) as i32;

            b &= (1 << nbits) - 1;
            /* loop over the next block */
            imax = i + nblock;
            if imax > nx {
                imax = nx;
            }
            if fs < 0 {
                /* low-entropy case, all zero differences */
                while i < imax {
                    output[i] = lastpix as c_uchar;
                    i += 1;
                }
            } else if fs == fsmax {
                /* high-entropy case, directly coded pixel values */
                while i < imax {
                    k = bbits - nbits;
                    diff = b.wrapping_shl(k as u32);
                    k -= 8;
                    while k >= 0 {
                        b = input[c_current] as u32;
                        c_current += 1;
                        diff |= b << k;
                        k -= 8
                    }
                    if nbits > 0 {
                        b = input[c_current] as u32;
                        c_current += 1;
                        diff |= b >> (-k);
                        b &= (1 << nbits) - 1;
                    } else {
                        b = 0;
                    }
                    /*
                     * undo mapping and differencing
                     * Note that some of these operations will overflow the
                     * unsigned int arithmetic -- that's OK, it all works
                     * out to give the right answers in the output file.
                     */
                    if (diff & 1) == 0 {
                        diff >>= 1;
                    } else {
                        diff = !(diff >> 1);
                    }
                    output[i] = diff.wrapping_add(lastpix) as c_uchar;
                    lastpix = output[i] as u32;
                    i += 1;
                }
            } else {
                /* normal case, Rice coding */
                while i < imax {
                    /* count number of leading zeros */
                    while b == 0 {
                        nbits += 8;

                        b = input[c_current] as u32;
                        c_current += 1;
                    }
                    nzero = nbits - NONZERO_COUNT[b as usize];
                    nbits -= nzero + 1;
                    /* flip the leading one-bit */
                    b ^= 1 << nbits;
                    /* get the FS trailing bits */
                    nbits -= fs;
                    while nbits < 0 {
                        b = (b << 8) | (input[c_current] as u32);

                        c_current += 1;
                        nbits += 8;
                    }
                    diff = ((nzero as u32) << fs) | (b >> nbits);
                    b &= (1 << nbits) - 1;

                    /* undo mapping and differencing */
                    if (diff & 1) == 0 {
                        diff >>= 1;
                    } else {
                        diff = !(diff >> 1);
                    }
                    output[i] = diff.wrapping_add(lastpix) as c_uchar;
                    lastpix = output[i] as u32;
                    i += 1;
                }
            }
            if c_current > input.len() {
                (self.log_fn)("decompression error: hit end of compressed byte stream");
                return Err(DecodeError::EndOfBuffer);
            }
        }
        if c_current < input.len() {
            (self.log_fn)("decompression warning: unused bytes at end of compressed buffer");
        }

        Ok(())
    }
}
