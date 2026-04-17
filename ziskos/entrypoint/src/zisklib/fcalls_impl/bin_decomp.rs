/// Proxy entry point: reads `len_x` and `x[0..len_x]` from `parameters`, writes the
/// MSB-first bit decomposition into `results`, and returns the number of results written.
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

/// Returns the MSB-first binary decomposition of `x` (a little-endian `[u64]` of `len_x` limbs).
/// For zero input the result is `[0]`.
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

    // If x is zero, we return a single bit of 0
    if decomposition.is_empty() {
        decomposition.push(0);
    }

    decomposition
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin_decomp_zero() {
        let x = [0];
        let params = [x.len() as u64, x[0]];
        let mut result = [0, 0];
        fcall_bin_decomp(&params, &mut result);
        let expected_len = 1;
        let expected_bits = [0];

        assert_eq!(result[0], expected_len);
        assert_eq!(result[1], expected_bits[0]);
    }

    #[test]
    fn test_bin_decomp_one() {
        let x = [1];
        let params = [x.len() as u64, x[0]];
        let mut result = [0, 0];
        fcall_bin_decomp(&params, &mut result);
        let expected_len = 1;
        let expected_bits = [1];

        assert_eq!(result[0], expected_len);
        assert_eq!(result[1], expected_bits[0]);
    }

    #[test]
    fn test_bin_decomp_two() {
        let x = [2];
        let params = [x.len() as u64, x[0]];
        let mut result = [0, 0, 0];
        fcall_bin_decomp(&params, &mut result);
        let expected_len = 2;
        let expected_bits = [1, 0];

        assert_eq!(result[0], expected_len);
        assert_eq!(result[1..(1 + expected_len as usize)], expected_bits);
    }

    #[test]
    fn test_bin_decomp_big() {
        let x = [1, 2, 3, 4];
        let params = [x.len() as u64, x[0], x[1], x[2], x[3]];
        let mut result = [0; 196];
        fcall_bin_decomp(&params, &mut result);
        let expected_len = 195;
        let expected_bits = [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ];

        assert_eq!(result[0], expected_len);
        assert_eq!(result[1..(1 + expected_len as usize)], expected_bits);
    }
}
