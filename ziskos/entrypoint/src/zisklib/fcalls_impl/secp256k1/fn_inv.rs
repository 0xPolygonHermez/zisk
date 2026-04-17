use lazy_static::lazy_static;
use num_bigint::BigUint;

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};

use super::N;

/// Perform the inversion of a NON-ZERO scalar field element in Fn
pub fn fcall_secp256k1_fn_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a: &[u64; 4] = &params[0..4].try_into().unwrap();

    // Perform the inversion using fn inversion
    let inv = secp256k1_fn_inv(a);

    // Store the result
    results[0..4].copy_from_slice(&inv);

    4
}

fn secp256k1_fn_inv(a: &[u64; 4]) -> [u64; 4] {
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

    fn secp256k1_fn_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
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
        fcall_secp256k1_fn_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }

    #[test]
    fn test_inv() {
        let x = [0xf9ee4256a589409f, 0xa21a3985f17502d0, 0xb3eb393d00dc480c, 0x142def02c537eced];
        let expected_inv =
            [0x32fe23e91aa741a1, 0x204b2da7afd93e75, 0x39b0bef6b00ec8b0, 0x7a0f1a7146326666];

        let mut results = [0; 4];
        fcall_secp256k1_fn_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
        assert_eq!(secp256k1_fn_mul(&x, &results), [1, 0, 0, 0]);

        let x = [0x3623dfe3727a53ca, 0x9834d5ea5c40a9dd, 0x3b13b13b13b13b13, 0x13b13b13b13b13b1];
        let expected_inv =
            [0x000000000000000d, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000];

        let mut results = [0; 4];
        fcall_secp256k1_fn_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
        assert_eq!(secp256k1_fn_mul(&x, &results), [1, 0, 0, 0]);
    }
}
