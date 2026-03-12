use num_integer::Integer;

use super::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};

/// Perform the division between two 256-bit non-zero numbers
pub fn fcall_big_int256_div(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a = &params[0..4].try_into().unwrap();
    let b = &params[4..8].try_into().unwrap();

    // Perform the division
    let (quotient, remainder) = big_int256_div(a, b);

    // Store the result
    results[0..4].copy_from_slice(&quotient);
    results[4..8].copy_from_slice(&remainder);

    8
}

pub fn big_int256_div(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let (quotient, remainder) = a_big.div_rem(&b_big);
    (n_u64_digits_from_biguint::<4>(&quotient), n_u64_digits_from_biguint::<4>(&remainder))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div() {
        let a = [0x16b12176aedd308e, 0x9d331c2b34766fc9, 0xb7f85b22001249e, 0x3b4e3fc5e0d8b014];
        let b = [0x16b12176aedd308e, 0x9d331c2b34766fc9, 0xb7f85b22001249e, 0x0];
        let params = [a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]];
        let mut results = [0; 8];
        fcall_big_int256_div(&params, &mut results);
        let expected_quo = [0x2868ebf5edfaecd5, 0x5, 0x0, 0x0];
        let expected_rem = [0xdbb84a86764e268, 0xfd48d6ec2b636246, 0xadbb6db4207ffb8, 0x0];

        assert_eq!(results[0..4], expected_quo);
        assert_eq!(results[4..8], expected_rem);
    }
}
