//! Native-side implementation of the secp256r1 scalar field inverse fcall.

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};

use super::constants::N;

/// Computes `x⁻¹ mod n` where `n` is the order of the secp256r1 scalar field.
/// Input: 4 little-endian `u64` limbs (the scalar `x`, assumed non-zero).
/// Output: 4 little-endian `u64` limbs (the inverse).
pub fn fcall_secp256r1_fn_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a: &[u64; 4] = &params[0..4].try_into().unwrap();

    // Perform the inversion using fn inversion
    let inv = secp256r1_fn_inv(a);

    // Store the result
    results[0..4].copy_from_slice(&inv);

    4
}

fn secp256r1_fn_inv(a: &[u64; 4]) -> [u64; 4] {
    let a_big = biguint_from_u64_digits(a);
    let inv = a_big.modinv(&N);
    match inv {
        Some(inverse) => n_u64_digits_from_biguint(&inverse),
        None => panic!("Inverse does not exist"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secp256r1_fn_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
        let a_big = biguint_from_u64_digits(a);
        let b_big = biguint_from_u64_digits(b);
        let ab_big = (a_big * b_big) % &*N;
        n_u64_digits_from_biguint::<4>(&ab_big)
    }

    #[test]
    fn test_inv_one() {
        let x = [1, 0, 0, 0];
        let expected_inv = [1, 0, 0, 0];

        let mut results = [0; 4];
        fcall_secp256r1_fn_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }

    #[test]
    fn test_inv() {
        let x = [0xf9ee4256a589409f, 0xa21a3985f17502d0, 0xb3eb393d00dc480c, 0x142def02c537eced];
        let expected_inv =
            [0x7450938531a554a4, 0x49a5e61e420cf950, 0x5e5e8115e302f1dd, 0xe4bac2152faee1f6];

        let mut results = [0; 4];
        fcall_secp256r1_fn_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
        assert_eq!(secp256r1_fn_mul(&x, &results), [1, 0, 0, 0]);
    }
}
