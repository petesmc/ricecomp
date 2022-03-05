fn ffpmsg(_m: &str) {}

pub const EOF: i32 = -1;

/*
  The following code was written by Richard White at STScI and made
  available for use in CFITSIO in July 1999.  These routines were
  originally contained in 2 source files: rcomp.c and rdecomp.c,
  and the 'include' file now called ricecomp.h was originally called buffer.h.

  Note that beginning with CFITSIO v3.08, EOB checking was removed to improve
  speed, and so now the input compressed bytes buffers must have been
  allocated big enough so that they will never be overflowed. A simple
  rule of thumb that guarantees the buffer will be large enough is to make
  it 1% larger than the size of the input array of pixels that are being
  compressed.

*/

/*----------------------------------------------------------*/
/*                                                          */
/*    START OF SOURCE FILE ORIGINALLY CALLED rcomp.c        */
/*                                                          */
/*----------------------------------------------------------*/
/* @(#) rcomp.c 1.5 99/03/01 12:40:27 */
/* rcomp.c	Compress image line using
 *		(1) Difference of adjacent pixels
 *		(2) Rice algorithm coding
 *
 * Returns number of bytes written to code buffer or
 * -1 on failure
 */

/*
 * nonzero_count is lookup table giving number of bits in 8-bit values not including
 * leading zeros used in fits_rdecomp, fits_rdecomp_short and fits_rdecomp_byte
 */
pub const NONZERO_COUNT: [i32; 256] = [
    0, 1, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
];

pub type BufferT = u8;

pub struct Buffer {
    bitbuffer: i32,  /* bit buffer			*/
    bits_to_go: i32, /* bits to go in buffer	*/
    current: usize,  /* current position in buffer	*/
    //end: usize,		    /* end of buffer		*/
    buffer: Vec<u8>,
}

// #define putcbuf(c,mf) 	((*(mf->current)++ = c), 0)
// TODO redefine as macro?
pub fn putcbuf(c: i32, mf: &mut Buffer) {
    mf.buffer.push(c as u8);
    //mf.buffer[mf.current] = c as u8;
    mf.current += 1;
}

#[derive(Debug)]
pub enum DecodeError {
    EndOfBuffer,
    ZeroSizeInput,
}

/* this routine used to be called 'rcomp'  (WDP)  */
/*---------------------------------------------------------------------------*/
pub fn fits_rcomp(
    a: &[i32], /* input array			*/
    nx: usize, /* number of input pixels	*/
    nblock: usize,
) -> Result<Vec<u8>, DecodeError> /* coding block size		*/
{
    if a.is_empty() || nblock == 0 {
        return Err(DecodeError::ZeroSizeInput);
    }

    //*buffer = &bufmem;

    /* int bsize;  */
    let mut _i: i32;
    let mut _j: i32;

    let mut nextpix: i32;
    let mut pdiff: i32;

    let mut v: i32;
    let mut fs: i32;
    let mut fsmask: i32;
    let mut top: i32;
    let mut _fsmax: i32;
    let _fsbits: i32;

    let mut lbitbuffer: i32;
    let mut lbits_to_go: i32;

    let mut psum: u32;
    let mut pixelsum: f64;
    let mut dpsum: f64;

    //unsigned int *diff;

    /*
     * Original size of each pixel (bsize, bytes) and coding block
     * size (nblock, pixels)
     * Could make bsize a parameter to allow more efficient
     * compression of short & byte images.
     */
    /*    bsize = 4;   */

    /*    nblock = 32; now an input parameter*/
    /*
     * From bsize derive:
     * FSBITS = # bits required to store FS
     * FSMAX = maximum value for FS
     * BBITS = bits/pixel for direct coding
     */

    /*
    switch (bsize) {
    case 1:
    fsbits = 3;
    fsmax = 6;
    break;
    case 2:
    fsbits = 4;
    fsmax = 14;
    break;
    case 4:
    fsbits = 5;
    fsmax = 25;
    break;
    default:
        ffpmsg("rdecomp: bsize must be 1, 2, or 4 bytes");
    return(-1);
    }
    */

    /* move out of switch block, to tweak performance */
    let fsbits: i32 = 5;
    let fsmax: i32 = 25;

    let bbits: i32 = 1 << fsbits;

    /*
     * Set up buffer pointers
     */
    let mut buffer: Buffer = Buffer {
        current: 0,
        bits_to_go: 8,
        bitbuffer: 0,
        buffer: Vec::with_capacity(nx * 4),
    };

    /*
     * array for differences mapped to non-negative values
     */
    let mut diff: Vec<u32> = vec![0; nblock];

    /*
     * Code in blocks of nblock pixels
     */
    start_outputing_bits(&mut buffer);

    /* write out first int value to the first 4 bytes of the buffer */
    if output_nbits(&mut buffer, a[0], 32) == EOF {
        ffpmsg("rice_encode: end of buffer");
        return Err(DecodeError::EndOfBuffer);
    }

    let mut lastpix: i32 = a[0]; /* the first difference will always be zero */

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
            nextpix = a[i + j];
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
            if output_nbits(&mut buffer, fsmax + 1, fsbits) == EOF {
                ffpmsg("rice_encode: end of buffer");
                return Err(DecodeError::EndOfBuffer);
            }

            for j in 0..thisblock {
                if output_nbits(&mut buffer, diff[j] as i32, bbits) == EOF {
                    ffpmsg("rice_encode: end of buffer");
                    return Err(DecodeError::EndOfBuffer);
                }
            }
        } else if fs == 0 && pixelsum == 0.0 {
            /*
             * special low entropy case when FS = 0 and pixelsum=0 (all
             * pixels in block are zero.)
             * Output a 0 and return
             */
            if output_nbits(&mut buffer, 0, fsbits) == EOF {
                ffpmsg("rice_encode: end of buffer");
                return Err(DecodeError::EndOfBuffer);
            }
        } else {
            /* normal case: not either very high or very low entropy */
            if output_nbits(&mut buffer, fs + 1, fsbits) == EOF {
                ffpmsg("rice_encode: end of buffer");
                return Err(DecodeError::EndOfBuffer);
            }
            fsmask = (1 << fs) - 1;
            /*
             * local copies of bit buffer to improve optimization
             */
            lbitbuffer = buffer.bitbuffer;
            lbits_to_go = buffer.bits_to_go;
            for j in 0..thisblock {
                v = diff[j] as i32;
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
                    putcbuf(lbitbuffer & 0xff, &mut buffer);

                    top -= lbits_to_go;
                    while top >= 8 {
                        putcbuf(0, &mut buffer);
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
                        putcbuf((lbitbuffer >> (-lbits_to_go)) & 0xff, &mut buffer);
                        lbits_to_go += 8;
                    }
                }
            }

            buffer.bitbuffer = lbitbuffer;
            buffer.bits_to_go = lbits_to_go;
        }
    }
    done_outputing_bits(&mut buffer);

    /*
     * return number of bytes used
     */
    Ok(buffer.buffer)
}
/*---------------------------------------------------------------------------*/

/*---------------------------------------------------------------------------*/
/* bit_output.c
 *
 * Bit output routines
 * Procedures return zero on success, EOF on end-of-buffer
 *
 * Programmer: R. White     Date: 20 July 1998
 */

/* Initialize for bit output */

pub fn start_outputing_bits(buffer: &mut Buffer) {
    /*
     * Buffer is empty to start with
     */
    buffer.bitbuffer = 0;
    buffer.bits_to_go = 8;
}

/*---------------------------------------------------------------------------*/
/* Output N bits (N must be <= 32) */

pub fn output_nbits(buffer: &mut Buffer, bits: i32, n: i32) -> i32 {
    /* local copies */

    let mut n = n;
    /* AND mask for the right-most n bits */
    const MASK: [u32; 33] = [
        0, 0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff, 0x3fff,
        0x7fff, 0xffff, 0x1ffff, 0x3ffff, 0x7ffff, 0xfffff, 0x1fffff, 0x3fffff, 0x7fffff, 0xffffff,
        0x1ffffff, 0x3ffffff, 0x7ffffff, 0xfffffff, 0x1fffffff, 0x3fffffff, 0x7fffffff, 0xffffffff,
    ];

    /*
     * insert bits at end of bitbuffer
     */
    let mut lbitbuffer: i32 = buffer.bitbuffer;
    let mut lbits_to_go: i32 = buffer.bits_to_go;
    if lbits_to_go + n > 32 {
        /*
         * special case for large n: put out the top lbits_to_go bits first
         * note that 0 < lbits_to_go <= 8
         */
        lbitbuffer <<= lbits_to_go;
        /*	lbitbuffer |= (bits>>(n-lbits_to_go)) & ((1<<lbits_to_go)-1); */
        lbitbuffer |= (bits >> (n - lbits_to_go)) & (MASK[lbits_to_go as usize] as i32);
        putcbuf(lbitbuffer & 0xff, buffer);
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
        putcbuf((lbitbuffer >> (-lbits_to_go)) & 0xff, buffer);
        lbits_to_go += 8;
    }
    buffer.bitbuffer = lbitbuffer;
    buffer.bits_to_go = lbits_to_go;
    0
}
/*---------------------------------------------------------------------------*/
/* Flush out the last bits */

pub fn done_outputing_bits(buffer: &mut Buffer) -> i32 {
    if buffer.bits_to_go < 8 {
        putcbuf(buffer.bitbuffer << buffer.bits_to_go, buffer);

        /*	if (putcbuf(buffer->bitbuffer<<buffer->bits_to_go,buffer) == EOF)
                return(EOF);
        */
    }
    0
}

/*---------------------------------------------------------------------------*/
/*----------------------------------------------------------*/
/*                                                          */
/*    START OF SOURCE FILE ORIGINALLY CALLED rdecomp.c      */
/*                                                          */
/*----------------------------------------------------------*/

/* @(#) rdecomp.c 1.4 99/03/01 12:38:41 */
/* rdecomp.c	Decompress image line using
 *		(1) Difference of adjacent pixels
 *		(2) Rice algorithm coding
 *
 * Returns 0 on success or 1 on failure
 */

/*---------------------------------------------------------------------------*/
/* this routine used to be called 'rdecomp'  (WDP)  */

pub fn fits_rdecomp(
    c: &[u8],  /* input buffer			*/
    nx: usize, /* number of output pixels	*/
    nblock: usize,
) -> Result<Vec<u32>, DecodeError> /* coding block size		*/
{
    /* int bsize;  */

    let mut k: i32;
    let mut imax: usize;

    let mut nzero: i32;
    let mut fs: i32;
    let _cend: u8;

    let mut diff: u32;

    let _fsmax: i32;
    let _fsbits: i32;
    let _bbits: i32;

    let mut array: Vec<u32> = vec![0; nx as usize];

    /*
     * Original size of each pixel (bsize, bytes) and coding block
     * size (nblock, pixels)
     * Could make bsize a parameter to allow more efficient
     * compression of short & byte images.
     */
    /*    bsize = 4; */

    /*    nblock = 32; now an input parameter */
    /*
     * From bsize derive:
     * FSBITS = # bits required to store FS
     * FSMAX = maximum value for FS
     * BBITS = bits/pixel for direct coding
     */

    /*
    switch (bsize) {
    case 1:
    fsbits = 3;
    fsmax = 6;
    break;
    case 2:
    fsbits = 4;
    fsmax = 14;
    break;
    case 4:
    fsbits = 5;
    fsmax = 25;
    break;
    default:
    ffpmsg("rdecomp: bsize must be 1, 2, or 4 bytes");
    return 1;
    }
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
    let mut bytevalue: u8 = c[0];
    lastpix |= (bytevalue as u32) << 24;
    bytevalue = c[1];
    lastpix |= (bytevalue as u32) << 16;
    bytevalue = c[2];
    lastpix |= (bytevalue as u32) << 8;
    bytevalue = c[3];
    lastpix |= bytevalue as u32;

    let mut c_current: usize = 4;

    // cend = c + clen - 4;

    let mut b: u32 = c[c_current] as u32; /* bit buffer			*/
    //TODO
    c_current += 1;
    let mut nbits: i32 = 8; /* number of bits remaining in b	*/

    let mut i: usize = 0;
    while i < nx {
        /* get the FS value from first fsbits */
        nbits -= fsbits;
        while nbits < 0 {
            b = (b << 8) | c[c_current] as u32;
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
                array[i] = lastpix;
                i += 1;
            }
        } else if fs == fsmax {
            /* high-entropy case, directly coded pixel values */
            while i < imax {
                k = bbits - nbits;
                diff = b.wrapping_shl(k as u32);
                k -= 8;
                while k >= 0 {
                    b = c[c_current] as u32;
                    c_current += 1;
                    diff |= b << k;
                    k -= 8
                }
                if nbits > 0 {
                    b = c[c_current] as u32;
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
                array[i] = diff.wrapping_add(lastpix);
                lastpix = array[i];
                i += 1;
            }
        } else {
            /* normal case, Rice coding */
            while i < imax {
                /* count number of leading zeros */
                while b == 0 {
                    nbits += 8;

                    b = c[c_current] as u32;
                    c_current += 1;
                }
                nzero = nbits - NONZERO_COUNT[b as usize];
                nbits -= nzero + 1;
                /* flip the leading one-bit */
                b ^= 1 << nbits;
                /* get the FS trailing bits */
                nbits -= fs;
                while nbits < 0 {
                    b = (b << 8) | (c[c_current] as u32);

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
                array[i] = diff.wrapping_add(lastpix);
                lastpix = array[i];
                i += 1;
            }
        }
        if c_current > c.len() {
            ffpmsg("decompression error: hit end of compressed byte stream");
            return Err(DecodeError::EndOfBuffer);
        }
    }
    if c_current < c.len() {
        ffpmsg("decompression warning: unused bytes at end of compressed buffer");
    }
    Ok(array)
}
/*---------------------------------------------------------------------------*/

#[derive(Clone, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Data {
    pub d: Vec<i32>,
    //pub bs: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encode_works() {
        let inarray: [i32; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ];
        let outarray = fits_rcomp(&inarray, 32, 16).unwrap();
        assert_eq!(outarray.len(), 17);
    }

    #[test]
    fn decode_works() {
        let bs = 16;
        let inarray: [i32; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ];
        let outarray = fits_rcomp(&inarray, 32, bs).unwrap();

        let new_inarray = fits_rdecomp(&outarray, 32, bs).unwrap();
        let new_inarray: Vec<i32> = new_inarray.iter().map(|&x| x as i32).collect();
        assert_eq!(new_inarray.len(), 32);
        assert_eq!(inarray.to_vec(), new_inarray);
    }

    #[test]
    fn uh_oh() {
        let inarray = [
            -1,
            -1,
            -33,
            -1,
            -1,
            -1,
            -1281,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -2555905,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -6,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -83886081,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -2555905,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -6,
            -1,
            -1,
            -1073741825,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1,
            -1281,
            -1,
            -1,
        ];
        let bs = 139;
        let outarray = fits_rcomp(&inarray, 141, bs).unwrap();

        let new_inarray = fits_rdecomp(&outarray, 141, bs).unwrap();
        let new_inarray: Vec<i32> = new_inarray.iter().map(|&x| x as i32).collect();
        assert_eq!(new_inarray.len(), 141);
        assert_eq!(inarray.to_vec(), new_inarray);
    }
}
