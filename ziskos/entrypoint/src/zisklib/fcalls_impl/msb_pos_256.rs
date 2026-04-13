/// Finds the limb index and bit position of the most significant set bit among `n`
/// concatenated 256-bit values in `params`. Panics if all values are zero.
pub fn fcall_msb_pos_256(params: &[u64], results: &mut [u64]) -> i64 {
    let n = params[0] as usize;

    let (limb, bit) = msb_pos_256(&params[1..], n);

    results[0] = limb as u64;
    results[1] = bit as u64;
    2
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msb_pos_256_1() {
        let params = [
            1, // n = 1
            1, 0, 0, 0, // input
        ];
        let mut results = [0u64; 2];
        fcall_msb_pos_256(&params, &mut results);
        assert_eq!(results[0], 0); // limb index
        assert_eq!(results[1], 0); // bit index

        let params = [
            1, // n = 1
            2, 0, 0, 0, // input
        ];
        let mut results = [0u64; 2];
        fcall_msb_pos_256(&params, &mut results);
        assert_eq!(results[0], 0); // limb index
        assert_eq!(results[1], 1); // bit index

        let params = [
            1, // n = 1
            1, 1, 0, 0, // input
        ];
        let mut results = [0u64; 2];
        fcall_msb_pos_256(&params, &mut results);
        assert_eq!(results[0], 1); // limb index
        assert_eq!(results[1], 0); // bit index

        let params = [
            1, // n = 1
            1, 2, 3, 2, // input
        ];
        let mut results = [0u64; 2];
        fcall_msb_pos_256(&params, &mut results);
        assert_eq!(results[0], 3); // limb index
        assert_eq!(results[1], 1); // bit index
    }

    #[test]
    fn test_msb_pos_256_2() {
        let params = [
            2, // n = 2
            0, 0, 0, 0, // input 1
            1, 0, 0, 0, // input 2
        ];
        let mut results = [0u64; 2];
        fcall_msb_pos_256(&params, &mut results);
        assert_eq!(results[0], 0); // limb index
        assert_eq!(results[1], 0); // bit index
    }

    #[test]
    fn test_msb_pos_256_3() {
        let params = [
            3, // n = 3
            0, 0, 0, 2, // input 1
            0, 0, 0, 0, // input 2
            0, 0, 2, 0, // input 3
        ];
        let mut results = [0u64; 2];
        fcall_msb_pos_256(&params, &mut results);
        assert_eq!(results[0], 3); // limb index
        assert_eq!(results[1], 1); // bit index
    }
}
