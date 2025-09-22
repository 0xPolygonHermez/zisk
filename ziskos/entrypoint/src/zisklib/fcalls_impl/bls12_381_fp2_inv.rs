use lazy_static::lazy_static;
use num_bigint::BigUint;

use super::bls12_381_fp_inv::{
    bls12_381_fp_add, bls12_381_fp_dbl, bls12_381_fp_inv, bls12_381_fp_mul, bls12_381_fp_neg,
    bls12_381_fp_square, bls12_381_fp_sub,
};

lazy_static! {
    pub static ref P: BigUint = BigUint::parse_bytes(
        b"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab",
        16
    )
    .unwrap();
}

/// Perform the inversion of a non-zero field element in Fp2
pub fn fcall_bls12_381_fp2_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a = &params[0..12].try_into().unwrap();

    // Perform the inversion using fp2 inversion
    let inv = bls12_381_fp2_inv(a);

    // Store the result
    results[0..12].copy_from_slice(&inv);

    12
}

pub fn bls12_381_fp2_inv(a: &[u64; 12]) -> [u64; 12] {
    let real = &a[0..6].try_into().unwrap();
    let imaginary = &a[6..12].try_into().unwrap();

    // Perform the inversion using fp inversion
    let denominator =
        bls12_381_fp_add(&bls12_381_fp_mul(real, real), &bls12_381_fp_mul(imaginary, imaginary));
    let denominator = bls12_381_fp_inv(&denominator);

    let inv_real = bls12_381_fp_mul(real, &denominator);
    let inv_imaginary = bls12_381_fp_mul(&bls12_381_fp_neg(imaginary), &denominator);

    [inv_real, inv_imaginary].concat().try_into().unwrap()
}

pub(crate) fn bls12_381_fp2_dbl(a: &[u64; 12]) -> [u64; 12] {
    let a_real = &a[0..6].try_into().unwrap();
    let a_imaginary = &a[6..12].try_into().unwrap();

    let real_part = bls12_381_fp_add(a_real, a_real);
    let imaginary_part = bls12_381_fp_add(a_imaginary, a_imaginary);

    [real_part, imaginary_part].concat().try_into().unwrap()
}

pub(crate) fn bls12_381_fp2_sub(a: &[u64; 12], b: &[u64; 12]) -> [u64; 12] {
    let a_real = &a[0..6].try_into().unwrap();
    let a_imaginary = &a[6..12].try_into().unwrap();
    let b_real = &b[0..6].try_into().unwrap();
    let b_imaginary = &b[6..12].try_into().unwrap();

    let real_part = bls12_381_fp_sub(a_real, b_real);
    let imaginary_part = bls12_381_fp_sub(a_imaginary, b_imaginary);

    [real_part, imaginary_part].concat().try_into().unwrap()
}

pub(crate) fn bls12_381_fp2_mul(a: &[u64; 12], b: &[u64; 12]) -> [u64; 12] {
    let a_real = &a[0..6].try_into().unwrap();
    let a_imaginary = &a[6..12].try_into().unwrap();
    let b_real = &b[0..6].try_into().unwrap();
    let b_imaginary = &b[6..12].try_into().unwrap();

    let real_part = bls12_381_fp_sub(
        &bls12_381_fp_mul(a_real, b_real),
        &bls12_381_fp_mul(a_imaginary, b_imaginary),
    );
    let imaginary_part = bls12_381_fp_add(
        &bls12_381_fp_mul(a_real, b_imaginary),
        &bls12_381_fp_mul(a_imaginary, b_real),
    );

    [real_part, imaginary_part].concat().try_into().unwrap()
}

pub(crate) fn bls12_381_fp2_square(a: &[u64; 12]) -> [u64; 12] {
    let a_real = &a[0..6].try_into().unwrap();
    let a_imaginary = &a[6..12].try_into().unwrap();

    let real_part =
        bls12_381_fp_sub(&bls12_381_fp_square(a_real), &bls12_381_fp_square(a_imaginary));
    let imaginary_part = bls12_381_fp_dbl(&bls12_381_fp_mul(a_real, a_imaginary));

    [real_part, imaginary_part].concat().try_into().unwrap()
}

pub(crate) fn bls12_381_fp2_scalar_mul(a: &[u64; 12], b: &[u64; 6]) -> [u64; 12] {
    let a_real = &a[0..6].try_into().unwrap();
    let a_imaginary = &a[6..12].try_into().unwrap();
    let b = &b[0..6].try_into().unwrap();

    let real_part = bls12_381_fp_mul(a_real, b);
    let imaginary_part = bls12_381_fp_mul(a_imaginary, b);

    [real_part, imaginary_part].concat().try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_one() {
        let x = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let expected_inv = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let mut results = [0; 12];
        fcall_bls12_381_fp2_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }

    #[test]
    fn test_inv() {
        let x = [
            0x49b4b9e2ffd3bf5a,
            0x6bc7632c9e4047a7,
            0x805d19211a7dc450,
            0x41c84ac8cfa40667,
            0xcbc8271a6d95e07f,
            0x167ed52ad9b8dc52,
            0x9919e620d143515b,
            0x808a3f274c49a6c7,
            0xd65c110346cb2c1b,
            0x8cd2c11ad5206061,
            0x791b9ace70502ab1,
            0x7f958516727acdd,
        ];
        let expected_inv = [
            0x55aa5b187f77e83e,
            0xff523f3ab3ac46a6,
            0xf686d520afbeb578,
            0xb1664497d371019b,
            0xcfef6ce72c61e835,
            0x1474b2da727c6dfe,
            0x5730d5d619884057,
            0xd42b3decc96db687,
            0x8abb9a0eed22a8a3,
            0xd2f92c46b24958f7,
            0x8ab323bd7384ca05,
            0x1859d94eddac5b45,
        ];

        let mut results = [0; 12];
        fcall_bls12_381_fp2_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }
}
