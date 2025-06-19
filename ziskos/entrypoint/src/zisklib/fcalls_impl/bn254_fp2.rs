use lazy_static::lazy_static;
use num_bigint::BigUint;

use super::bn254_fp::{
    bn254_fp_add, bn254_fp_dbl, bn254_fp_inv, bn254_fp_mul, bn254_fp_neg, bn254_fp_square,
    bn254_fp_sub,
};

lazy_static! {
    static ref P: BigUint = BigUint::parse_bytes(
        b"30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47",
        16
    )
    .unwrap();
}

/// Perform the inversion of a non-zero field element in Fp2
pub fn fcall_bn254_fp2_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a = &params[0..8].try_into().unwrap();
    let inv = bn254_fp2_inv(a);

    // Store the result
    results[0..8].copy_from_slice(&inv);

    8
}

pub fn bn254_fp2_inv(a: &[u64; 8]) -> [u64; 8] {
    let real = &a[0..4].try_into().unwrap();
    let imaginary = &a[4..8].try_into().unwrap();

    // Perform the inversion using fp inversion
    let denominator = bn254_fp_add(&bn254_fp_mul(real, real), &bn254_fp_mul(imaginary, imaginary));
    let denominator = bn254_fp_inv(&denominator);

    let inv_real = bn254_fp_mul(real, &denominator);
    let inv_imaginary = bn254_fp_mul(&bn254_fp_neg(imaginary), &denominator);

    [
        inv_real[0],
        inv_real[1],
        inv_real[2],
        inv_real[3],
        inv_imaginary[0],
        inv_imaginary[1],
        inv_imaginary[2],
        inv_imaginary[3],
    ]
}

pub fn bn254_fp2_dbl(a: &[u64; 8]) -> [u64; 8] {
    let a_real = &a[0..4].try_into().unwrap();
    let a_imaginary = &a[4..8].try_into().unwrap();

    let real_part = bn254_fp_add(a_real, a_real);
    let imaginary_part = bn254_fp_add(a_imaginary, a_imaginary);

    [
        real_part[0],
        real_part[1],
        real_part[2],
        real_part[3],
        imaginary_part[0],
        imaginary_part[1],
        imaginary_part[2],
        imaginary_part[3],
    ]
}

pub fn bn254_fp2_sub(a: &[u64; 8], b: &[u64; 8]) -> [u64; 8] {
    let a_real = &a[0..4].try_into().unwrap();
    let a_imaginary = &a[4..8].try_into().unwrap();
    let b_real = &b[0..4].try_into().unwrap();
    let b_imaginary = &b[4..8].try_into().unwrap();

    let real_part = bn254_fp_sub(a_real, b_real);
    let imaginary_part = bn254_fp_sub(a_imaginary, b_imaginary);

    [
        real_part[0],
        real_part[1],
        real_part[2],
        real_part[3],
        imaginary_part[0],
        imaginary_part[1],
        imaginary_part[2],
        imaginary_part[3],
    ]
}

pub fn bn254_fp2_mul(a: &[u64; 8], b: &[u64; 8]) -> [u64; 8] {
    let a_real = &a[0..4].try_into().unwrap();
    let a_imaginary = &a[4..8].try_into().unwrap();
    let b_real = &b[0..4].try_into().unwrap();
    let b_imaginary = &b[4..8].try_into().unwrap();

    let real_part =
        bn254_fp_sub(&bn254_fp_mul(a_real, b_real), &bn254_fp_mul(a_imaginary, b_imaginary));
    let imaginary_part =
        bn254_fp_add(&bn254_fp_mul(a_real, b_imaginary), &bn254_fp_mul(a_imaginary, b_real));

    [
        real_part[0],
        real_part[1],
        real_part[2],
        real_part[3],
        imaginary_part[0],
        imaginary_part[1],
        imaginary_part[2],
        imaginary_part[3],
    ]
}

pub fn bn254_fp2_square(a: &[u64; 8]) -> [u64; 8] {
    let a_real = &a[0..4].try_into().unwrap();
    let a_imaginary = &a[4..8].try_into().unwrap();

    let real_part = bn254_fp_sub(&bn254_fp_square(a_real), &bn254_fp_square(a_imaginary));
    let imaginary_part = bn254_fp_dbl(&bn254_fp_mul(a_real, a_imaginary));

    [
        real_part[0],
        real_part[1],
        real_part[2],
        real_part[3],
        imaginary_part[0],
        imaginary_part[1],
        imaginary_part[2],
        imaginary_part[3],
    ]
}

pub fn bn254_fp2_scalar_mul(a: &[u64; 8], b: &[u64; 4]) -> [u64; 8] {
    let a_real = &a[0..4].try_into().unwrap();
    let a_imaginary = &a[4..8].try_into().unwrap();
    let b = &b[0..4].try_into().unwrap();

    let real_part = bn254_fp_mul(a_real, b);
    let imaginary_part = bn254_fp_mul(a_imaginary, b);

    [
        real_part[0],
        real_part[1],
        real_part[2],
        real_part[3],
        imaginary_part[0],
        imaginary_part[1],
        imaginary_part[2],
        imaginary_part[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_one() {
        let x = [1, 0, 0, 0, 0, 0, 0, 0];
        let expected_inv = [1, 0, 0, 0, 0, 0, 0, 0];

        let mut results = [0; 8];
        fcall_bn254_fp2_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }

    #[test]
    fn test_inv() {
        let x = [
            0xa4528921da9661b8,
            0xc13514a2f09d4f06,
            0x52406705a0d612b8,
            0x2b02b26b72efef38,
            0xb64cd3ecb5b08b28,
            0xe29c6143da89de45,
            0xdfa4f8b46115f7f6,
            0x17abb41fc8d1b2c7,
        ];
        let expected_inv = [
            0x163d11f5aa617bfc,
            0x825bc78934e518e5,
            0x31485988143cff2e,
            0x0551d3643b94a0ba,
            0xbd2738b4b0c67843,
            0xbed5ac50b31d3cef,
            0x516d2e7c293eef52,
            0x302d79e76ed154c1,
        ];

        let mut results = [0; 8];
        fcall_bn254_fp2_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }
}
