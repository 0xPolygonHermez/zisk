pub fn fcall_msb_pos_256(params: &[u64], results: &mut [u64]) -> i64 {
    let n = params[0] as usize;

    let (limb, bit) = msb_pos_256(&params[1..], n);

    results[0] = limb as u64;
    results[1] = bit as u64;
    2
}

// Q: Do we prefer constant time functions?
// Finds the most significant bit position among n 256-bit integers
// some of which may be zero, but not all
pub fn msb_pos_256(params: &[u64], n: usize) -> (usize, usize) {
    debug_assert!(params.len() >= n * 4, "Not enough data for {} inputs", n);

    for limb in (0..4).rev() {
        // Find max value at this limb position across all inputs
        let mut max_word = 0u64;
        for i in 0..n {
            let word = params[i * 4 + limb];
            if word > max_word {
                max_word = word;
            }
        }

        if max_word != 0 {
            return (limb, msb_pos(max_word));
        }
    }
    panic!("Invalid input: all values are zero");
}

// Q: Do we prefer constant time functions?
#[rustfmt::skip]
fn msb_pos(mut x: u64) -> usize {
    let mut pos = 0;
    if x >= 1 << 32 { x >>= 32; pos += 32; }
    if x >= 1 << 16 { x >>= 16; pos += 16; }
    if x >= 1 << 8  { x >>= 8;  pos += 8;  }
    if x >= 1 << 4  { x >>= 4;  pos += 4;  }
    if x >= 1 << 2  { x >>= 2;  pos += 2;  }
    if x >= 1 << 1  {           pos += 1;  }
    pos
}
