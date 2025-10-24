use lazy_static::lazy_static;
use num_bigint::BigUint;

use super::utils::{from_limbs_le, to_limbs_le};

lazy_static! {
    pub static ref P: BigUint = BigUint::parse_bytes(
        b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
        16
    )
    .unwrap();

    pub static ref P_HALF: BigUint = BigUint::parse_bytes(
        b"7fffffffffffffffffffffffffffffffffffffffffffffffffffffff7ffffe17",
        16
    )
    .unwrap();

    pub static ref P_DIV_4: BigUint = BigUint::parse_bytes(
        b"3fffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffff0c",
        16
    )
    .unwrap();

    pub static ref NQR: BigUint = BigUint::from(3u64); // First non-quadratic residue in Fp
}

pub fn fcall_secp256k1_fp_sqrt(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a: &[u64; 4] = &params[0..4].try_into().unwrap();
    let parity = params[4];

    // Perform the square root
    secp256k1_fp_sqrt(a, parity, results);

    5
}

fn secp256k1_fp_sqrt(a: &[u64; 4], parity: u64, results: &mut [u64]) {
    let a_big = from_limbs_le(a);

    // Attempt to compute the square root of a
    let mut sqrt = a_big.modpow(&P_DIV_4, &P);

    // Check if a is a quadratic residue
    let square = (&sqrt * &sqrt) % &*P;
    let a_is_qr = square == a_big;
    results[0] = a_is_qr as u64;
    if !a_is_qr {
        // To check that a is indeed a non-quadratic residue, we check that
        // a * NQR is a quadratic residue for some fixed known non-quadratic residue NQR
        let a_nqr = (a_big * &*NQR) % &*P;

        // Compute the square root of a * NQR
        let sqrt_nqr = a_nqr.modpow(&P_DIV_4, &P);

        results[1..5].copy_from_slice(&to_limbs_le::<4>(&sqrt_nqr));
        return;
    }

    // Flip the sqrt if needed to match the requested parity
    let sqrt_r = to_limbs_le::<4>(&sqrt);
    let sqrt_parity = (sqrt_r[0] & 1) as u64;
    if parity != sqrt_parity {
        sqrt = (&*P - &sqrt) % &*P;
    }

    results[1..5].copy_from_slice(&to_limbs_le::<4>(&sqrt));
}

#[cfg(test)]
mod tests {
    use super::*;

    const P_MINUS_ONE: [u64; 4] =
        [0xfffffffefffffc2e, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff];

    fn secp256k1_fp_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
        let a_big = from_limbs_le(a);
        let b_big = from_limbs_le(b);
        let ab_big = (a_big * b_big) % &*P;
        to_limbs_le::<4>(&ab_big)
    }

    #[test]
    fn test_sqrt_one() {
        let x = [1, 0, 0, 0];
        let parity = 1;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt = [1, 0, 0, 0];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..5].try_into().unwrap();
        assert_eq!(has_sqrt, 1);
        assert_eq!(sqrt, &expected_sqrt);
        assert_eq!(secp256k1_fp_mul(sqrt, sqrt), x);

        let parity = 0;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt = P_MINUS_ONE;

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sol = results[0];
        assert!(has_sol == 1);
        assert_eq!(results[1..5], expected_sqrt);
    }

    #[test]
    fn test_sqrt() {
        let x = [0x643764b2faa1592a, 0x4ac3ab52286f702a, 0x6591d88c833ffd4f, 0xc6fb7a1e514eac26];
        let parity = 0;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt =
            [0xa3d2fb0160f29df6, 0x3ebce4d565b52649, 0x4cdec0bf5c968639, 0x123e42087c415355];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..5].try_into().unwrap();
        assert_eq!(has_sqrt, 1);
        assert_eq!(sqrt, &expected_sqrt);
        assert_eq!(secp256k1_fp_mul(sqrt, sqrt), x);

        let parity = 1;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt =
            [0x5c2d04fd9f0d5e39, 0xc1431b2a9a4ad9b6, 0xb3213f40a36979c6, 0xedc1bdf783beacaa];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..5].try_into().unwrap();
        assert_eq!(has_sqrt, 1);
        assert_eq!(sqrt, &expected_sqrt);
        assert_eq!(secp256k1_fp_mul(sqrt, sqrt), x);
    }

    #[test]
    fn test_no_sqrt() {
        // We dont care about the parity bit if no sqrt

        let x = [0x643764b2faa1592c, 0x4ac3ab52286f702a, 0x6591d88c833ffd4f, 0xc6fb7a1e514eac26];
        let parity = 0;
        let params = [x[0], x[1], x[2], x[3], parity];
        let expected_sqrt =
            [0xdab2978e63122590, 0x5dc785c971480237, 0x87a60df9f92b07b9, 0x855b365e9f83d30d]; // sqrt(x * NQR)

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..5].try_into().unwrap();
        assert_eq!(has_sqrt, 0);
        assert_eq!(sqrt, &expected_sqrt);
        let nqr = to_limbs_le(&NQR);
        assert_eq!(secp256k1_fp_mul(sqrt, sqrt), secp256k1_fp_mul(&x, &nqr));

        let parity = 1;
        let params = [x[0], x[1], x[2], x[3], parity];

        let mut results = [0; 5];
        fcall_secp256k1_fp_sqrt(&params, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..5].try_into().unwrap();
        assert_eq!(has_sqrt, 0);
        assert_eq!(sqrt, &expected_sqrt);
        let nqr = to_limbs_le(&NQR);
        assert_eq!(secp256k1_fp_mul(sqrt, sqrt), secp256k1_fp_mul(&x, &nqr));
    }
}
