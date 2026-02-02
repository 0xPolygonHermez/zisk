pub fn fcall_bin_decomp(parameters: &[u64], results: &mut [u64]) -> i64 {
    let len_x = parameters[0] as usize;
    let x = &parameters[1..(1 + len_x)];

    let bits = bin_decomp(x, len_x);

    let len_bits = bits.len();
    results[0] = len_bits as u64;
    for i in 0..len_bits {
        results[1 + i] = bits[i] as u64;
    }
    (1 + len_bits) as i64
}

pub fn bin_decomp(x: &[u64], len_x: usize) -> Vec<u8> {
    let mut decomposition = Vec::new();
    let mut started = false;

    for i in (0..len_x).rev() {
        for bit_pos in (0..64).rev() {
            let bit = ((x[i] >> bit_pos) & 1) as u8;

            // Start recording once we hit the first 1 bit
            if !started && bit == 1 {
                started = true;
            }

            if started {
                decomposition.push(bit);
            }
        }
    }

    decomposition
}
