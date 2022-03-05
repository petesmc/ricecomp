#![no_main]
use libfuzzer_sys::fuzz_target;
extern crate ricecomp;
use crate::ricecomp::{fits_rcomp, fits_rdecomp, Data};


fuzz_target!(|data: Data| {
    let l = data.d.len();
   // let blocksz = if data.bs > 64 { (data.bs % 64)+1 } else {data.bs};
    //let blocksz = if blocksz < 1 { 1 } else { blocksz };
    let blocksz = 32;

    let comp_array = fits_rcomp(&data.d, l, blocksz as usize ); 
    match comp_array {
        Ok(v) => {
            let decomp_array = fits_rdecomp(&v,l, blocksz as usize).unwrap();
            let decomp_array: Vec<i32> = decomp_array.iter().map(|&x| x as i32).collect();

            assert_eq!(data.d, decomp_array);
        },
        Err(v) => {

        }
    }
    
});
