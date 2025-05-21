use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};

lazy_static! {
    pub static ref P: BigUint = BigUint::parse_bytes(
        b"30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47",
        16
    )
    .unwrap();
}

/// Perform the inversion of a non-zero field element in Fp2
pub fn bn254_fp2_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let real: &[u64; 4] = &params[0..4].try_into().unwrap();
    let imaginary: &[u64; 4] = &params[4..8].try_into().unwrap();

    // Perform the inversion using fp inversion
    let denominator = bn254_fp_add(&bn254_fp_mul(real, real), &bn254_fp_mul(imaginary, imaginary));
    let denominator = bn254_fp_inv(&denominator);

    let inv_real = bn254_fp_mul(real, &denominator);
    let inv_imaginary = bn254_fp_mul(&bn254_fp_neg(imaginary), &denominator);

    // Store the result
    results[0..4].copy_from_slice(&inv_real);
    results[4..8].copy_from_slice(&inv_imaginary);

    8
}

fn bn254_fp_add(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let b_big = from_limbs_le(b);
    let sum = (a_big + b_big) % &*P;
    to_limbs_le(&sum)
}

fn bn254_fp_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let b_big = from_limbs_le(b);
    let product = (a_big * b_big) % &*P;
    to_limbs_le(&product)
}

fn bn254_fp_neg(a: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let neg = &*P - a_big;
    to_limbs_le(&neg)
}

fn bn254_fp_inv(a: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let inv = a_big.modinv(&*P);
    match inv {
        Some(inverse) => to_limbs_le(&inverse),
        None => {
            // Handle the case where the inverse does not exist
            panic!("Inverse does not exist");
        }
    }
}

fn from_limbs_le(limbs: &[u64; 4]) -> BigUint {
    limbs.iter().rev().fold(BigUint::zero(), |acc, &limb| (acc << 64) + BigUint::from(limb))
}

fn to_limbs_le(value: &BigUint) -> [u64; 4] {
    let mut limbs = [0u64; 4];
    let mut _value = value.clone();
    for limb in limbs.iter_mut() {
        *limb = (_value.clone() & BigUint::from(u64::MAX)).to_u64().unwrap();
        _value >>= 64;
    }
    limbs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_one() {
        let x = [1, 0, 0, 0, 0, 0, 0, 0];
        let expected_inv = [1, 0, 0, 0, 0, 0, 0, 0];

        let mut results = [0; 8];
        bn254_fp2_inv(&x, &mut results);
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
        bn254_fp2_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }
}
