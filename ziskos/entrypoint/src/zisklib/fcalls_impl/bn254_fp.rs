use lazy_static::lazy_static;
use num_bigint::BigUint;

use super::utils::{from_limbs_le, to_limbs_le};

lazy_static! {
    pub static ref P: BigUint = BigUint::parse_bytes(
        b"30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47",
        16
    )
    .unwrap();
}

/// Perform the inversion of a non-zero field element in Fp
pub fn fcall_bn254_fp_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a: &[u64; 4] = &params[0..4].try_into().unwrap();

    // Perform the inversion using fp inversion
    let inv = bn254_fp_inv(a);

    // Store the result
    results[0..4].copy_from_slice(&inv);

    4
}

pub fn bn254_fp_add(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let b_big = from_limbs_le(b);
    let sum = (a_big + b_big) % &*P;
    to_limbs_le(&sum)
}

pub fn bn254_fp_dbl(a: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let double = (a_big.clone() + a_big) % &*P;
    to_limbs_le(&double)
}

pub fn bn254_fp_sub(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let b_big = from_limbs_le(b);
    let diff = if a_big >= b_big { (a_big - b_big) % &*P } else { ((a_big + &*P) - b_big) % &*P };
    to_limbs_le(&diff)
}

pub fn bn254_fp_neg(a: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let neg = &*P - a_big;
    to_limbs_le(&neg)
}

pub fn bn254_fp_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let b_big = from_limbs_le(b);
    let product = (a_big * b_big) % &*P;
    to_limbs_le(&product)
}

pub fn bn254_fp_square(a: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let square = (a_big.clone() * a_big) % &*P;
    to_limbs_le(&square)
}

pub fn bn254_fp_inv(a: &[u64; 4]) -> [u64; 4] {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_one() {
        let x = [1, 0, 0, 0];
        let expected_inv = [1, 0, 0, 0];

        let mut results = [0; 4];
        fcall_bn254_fp_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }

    #[test]
    fn test_inv() {
        let x = [0xf9ee4256a589409f, 0xa21a3985f17502d0, 0xb3eb393d00dc480c, 0x142def02c537eced];
        let expected_inv =
            [0x7258dab6e90d1680, 0x779f7ec5cad25c1d, 0xb9c114d250bcaa3c, 0x2525db1f6832d97d];

        let mut results = [0; 4];
        fcall_bn254_fp_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }
}
