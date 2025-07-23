use num_bigint::BigUint;

use super::utils::{from_limbs_le, to_limbs_le};

pub fn fcall_div_rem_256(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the inputs
    let x: &[u64; 4] = &params[0..4].try_into().unwrap();
    let y: &[u64; 4] = &params[4..8].try_into().unwrap();

    // Perform the division and get the result
    let (div, rem) = div_rem_256(x, y);

    // Store the result
    results[0..4].copy_from_slice(&div);
    results[4..8].copy_from_slice(&rem);

    8
}

fn div_rem_256(x: &[u64; 4], y: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    let y_big = from_limbs_le(y);
    if y_big == BigUint::ZERO {
        panic!("Division by zero");
    }

    let x_big = from_limbs_le(x);
    let div = x_big.clone() / y_big.clone();
    let rem = x_big % y_big;
    (to_limbs_le(&div), to_limbs_le(&rem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_one_by_one() {
        let x = [1, 0, 0, 0];
        let y = [1, 0, 0, 0];
        let params = [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]];

        let mut results = [0; 8];
        fcall_div_rem_256(&params, &mut results);
        let div = &results[0..4];
        let rem = &results[4..8];
        assert_eq!(div, &[1, 0, 0, 0]);
        assert_eq!(rem, &[0, 0, 0, 0]);
    }

    #[test]
    fn test_div_one_by_two() {
        let x = [1, 0, 0, 0];
        let y = [2, 0, 0, 0];
        let params = [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]];

        let mut results = [0; 8];
        fcall_div_rem_256(&params, &mut results);
        let div = &results[0..4];
        let rem = &results[4..8];
        assert_eq!(div, &[0, 0, 0, 0]);
        assert_eq!(rem, &[1, 0, 0, 0]);
    }

    #[test]
    fn test_div_complex() {
        let x = [1, 2, 3, 4];
        let y = [1, 2, 3, 3];
        let params = [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]];

        let mut results = [0; 8];
        fcall_div_rem_256(&params, &mut results);
        let div = &results[0..4];
        let rem = &results[4..8];
        assert_eq!(div, &[1, 0, 0, 0]);
        assert_eq!(rem, &[0, 0, 0, 1]);
    }
}
