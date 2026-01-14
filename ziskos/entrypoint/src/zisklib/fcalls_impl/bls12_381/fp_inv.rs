use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::Zero;

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};

use super::P;

/// Perform the inversion of a non-zero field element in Fp
pub fn fcall_bls12_381_fp_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a = &params[0..6].try_into().unwrap();

    // Perform the inversion using fp inversion
    let inv = bls12_381_fp_inv(a);

    // Store the result
    results[0..6].copy_from_slice(&inv);

    6
}

pub(crate) fn bls12_381_fp_inv(a: &[u64; 6]) -> [u64; 6] {
    let a_big = biguint_from_u64_digits(a);
    let inv = a_big.modinv(&P);
    match inv {
        Some(inverse) => n_u64_digits_from_biguint::<6>(&inverse),
        None => panic!("Inverse does not exist"),
    }
}

pub(crate) fn bls12_381_fp_add(a: &[u64; 6], b: &[u64; 6]) -> [u64; 6] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let sum = (a_big + b_big) % &*P;
    n_u64_digits_from_biguint::<6>(&sum)
}

pub(crate) fn bls12_381_fp_dbl(a: &[u64; 6]) -> [u64; 6] {
    let a_big = biguint_from_u64_digits(a);
    let double = (a_big << 1) % &*P;
    n_u64_digits_from_biguint::<6>(&double)
}

pub(crate) fn bls12_381_fp_neg(a: &[u64; 6]) -> [u64; 6] {
    let a_big = biguint_from_u64_digits(a);
    if a_big.is_zero() {
        return [0u64; 6];
    }
    let neg = &*P - a_big;
    n_u64_digits_from_biguint::<6>(&neg)
}

pub(crate) fn bls12_381_fp_sub(a: &[u64; 6], b: &[u64; 6]) -> [u64; 6] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let diff = if a_big >= b_big { a_big - b_big } else { (a_big + &*P) - b_big };
    n_u64_digits_from_biguint::<6>(&diff)
}

pub(crate) fn bls12_381_fp_mul(a: &[u64; 6], b: &[u64; 6]) -> [u64; 6] {
    let a_big = biguint_from_u64_digits(a);
    let b_big = biguint_from_u64_digits(b);
    let product = (a_big * b_big) % &*P;
    n_u64_digits_from_biguint::<6>(&product)
}

pub(crate) fn bls12_381_fp_square(a: &[u64; 6]) -> [u64; 6] {
    let a_big = biguint_from_u64_digits(a);
    let square = (&a_big * &a_big) % &*P;
    n_u64_digits_from_biguint::<6>(&square)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_one() {
        let x = [1, 0, 0, 0, 0, 0];
        let expected_inv = [1, 0, 0, 0, 0, 0];

        let mut results = [0; 6];
        fcall_bls12_381_fp_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }

    #[test]
    fn test_inv() {
        let x = [
            0x2d5f30c1d0577c56,
            0x29aabf4bbbb4b60a,
            0xf65faa3d6bda5044,
            0xa56da205ae4bf114,
            0x6ad30a8453e66eac,
            0x10a97e50d00668c,
        ];
        let expected_inv = [
            0x1d8053f2aed3d017,
            0x2912c6d8d7c59be0,
            0xea3af967ab741430,
            0xdc3cb17c3b332919,
            0x52a4afd74a0b5b20,
            0x12be47b0938a6ee1,
        ];

        let mut results = [0; 6];
        fcall_bls12_381_fp_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }
}
