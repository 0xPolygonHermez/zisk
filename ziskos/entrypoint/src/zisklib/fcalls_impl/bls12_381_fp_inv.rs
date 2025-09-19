use lazy_static::lazy_static;
use num_bigint::BigUint;

use super::utils::{from_limbs_le, to_limbs_le};

lazy_static! {
    pub static ref P: BigUint = BigUint::parse_bytes(
        b"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab",
        16
    )
    .unwrap();
}

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
    let a_big = from_limbs_le(a);
    let inv = a_big.modinv(&P);
    match inv {
        Some(inverse) => to_limbs_le(&inverse),
        None => {
            // Handle the case where the inverse does not exist
            panic!("Inverse does not exist");
        }
    }
}

pub(crate) fn bls12_381_fp_add(a: &[u64; 6], b: &[u64; 6]) -> [u64; 6] {
    let a_big = from_limbs_le(a);
    let b_big = from_limbs_le(b);
    let sum = (a_big + b_big) % &*P;
    to_limbs_le(&sum)
}

pub(crate) fn bls12_381_fp_neg(a: &[u64; 6]) -> [u64; 6] {
    let a_big = from_limbs_le(a);
    let neg = &*P - a_big;
    to_limbs_le(&neg)
}

pub(crate) fn bls12_381_fp_mul(a: &[u64; 6], b: &[u64; 6]) -> [u64; 6] {
    let a_big = from_limbs_le(a);
    let b_big = from_limbs_le(b);
    let product = (a_big * b_big) % &*P;
    to_limbs_le(&product)
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
