#include "ricecomp.c"

int main()
{
    size_t clen;   /* size of cbuf */
   // short *cbuf;   /* compressed data */
    clen = 800;
    unsigned char *cbuf;
    cbuf = calloc(clen, sizeof(unsigned char));

    //cbuf = (short *)calloc(clen, sizeof(unsigned char));
    //unsigned char cbuf[800];
    int nelem = 0;
    int nelem2 = 0; /* number of bytes */

     int idata[141] = {
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
        
    };

    int odata[141];
    nelem = fits_rcomp(idata, 141, (unsigned char *)cbuf, clen, 139);

    nelem2 = fits_rdecomp((unsigned char *)cbuf, clen, odata, 141, 139);

    // WARNING
    // idata != odata in the last 8 bytes
    // Can go past end of buffer
    /*
    while (b == 0)
				{
					nbits += 8;
					b = *c++;  ///// ###### <<<<<<<<<----
				}
    */
    

    return 0;
}