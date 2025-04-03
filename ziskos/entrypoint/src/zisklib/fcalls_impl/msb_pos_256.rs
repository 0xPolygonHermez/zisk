pub fn fcall_msb_pos_256(parameters: &[u64], results: &mut [u64]) -> i64 {
    // Check if the parameters are valid
    let x = &parameters[0..4].try_into().unwrap();
    let y = &parameters[4..8].try_into().unwrap();

    let (i, pos) = msb_pos_256(x, y);
    results[0] = i as u64;
    results[1] = pos as u64;
    2
}

// Q: Do we prefer constant time functions?
fn msb_pos_256(x: &[u64; 4], y: &[u64; 4]) -> (usize, usize) {
    for i in (0..4).rev() {
        if x[i] != 0 || y[i] != 0 {
            let word = if x[i] > y[i] { x[i] } else { y[i] };
            return (i, msb_pos(word));
        }
    }
    panic!("Invalid input: x and y are both zero");
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
