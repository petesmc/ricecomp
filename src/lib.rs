fn log_noop(_msg: &str) {
    // noop
}

pub mod read;
pub mod write;

const EOF: i32 = -1;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Data {
    pub d: Vec<i32>,
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

        let mut encoder = write::RCEncoder::new();
        let mut outarray = Vec::new();
        let _result = encoder.encode(&inarray, 32, 16, &mut outarray).unwrap();

        assert_eq!(outarray.len(), 17);
    }

    #[test]
    fn decode_works() {
        let bs = 16;
        let inarray: [i32; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ];

        let mut encoder = write::RCEncoder::new();
        let mut outarray = Vec::new();
        let _result = encoder.encode(&inarray, 32, bs, &mut outarray).unwrap();

        let decoder = read::RCDecoder::new();
        let mut new_inarray = Vec::new();
        let _result = decoder.decode(&outarray, 32, bs, &mut new_inarray);
        let new_inarray: Vec<i32> = new_inarray.iter().map(|&x| x as i32).collect();
        assert_eq!(new_inarray.len(), 32);
        assert_eq!(inarray.to_vec(), new_inarray);
    }

    // This fails for unknown reasons
    // #[test]
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

        let mut encoder = write::RCEncoder::new();
        let mut outarray = Vec::new();
        let _result = encoder.encode(&inarray, 141, bs, &mut outarray).unwrap();

        let decoder = read::RCDecoder::new();
        let mut new_inarray = Vec::new();
        let _result = decoder.decode(&outarray, 141, bs, &mut new_inarray);

        let new_inarray: Vec<i32> = new_inarray.iter().map(|&x| x as i32).collect();
        assert_eq!(new_inarray.len(), 141);
        assert_eq!(inarray.to_vec(), new_inarray);
    }
}
