mod round;
use round::blake3_round;

/// BLAKE3 simplified compresssion function
pub fn blake3_f(v: &mut [u32; 16], m: &[u32; 16]) {
    blake3_round(v, m, 0);
    blake3_round(v, m, 1);
    blake3_round(v, m, 2);
    blake3_round(v, m, 3);
    blake3_round(v, m, 4);
    blake3_round(v, m, 5);
    blake3_round(v, m, 6);
}
