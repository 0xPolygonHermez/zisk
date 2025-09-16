use lazy_static::lazy_static;
use num_bigint::BigUint;

use super::utils::{from_limbs_le, to_limbs_le};

lazy_static! {
    pub static ref P: BigUint = BigUint::parse_bytes(
        b"1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab",
        16
    )
    .unwrap();

    pub static ref P_HALF: BigUint = BigUint::parse_bytes(
        b"d0088f51cbff34d258dd3db21a5d66bb23ba5c279c2895fb39869507b587b120f55ffff58a9ffffdcff7fffffffd555",
        16
    )
    .unwrap();

    pub static ref P_DIV_4: BigUint = BigUint::parse_bytes(
        b"680447a8e5ff9a692c6e9ed90d2eb35d91dd2e13ce144afd9cc34a83dac3d8907aaffffac54ffffee7fbfffffffeaab",
        16
    )
    .unwrap();

    pub static ref NQR: BigUint = BigUint::from(2u64); // First non-quadratic residue in Fp
}

/// Computes the square root of a non-zero field element in Fp
pub fn fcall_bls12_381_fp_sqrt(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a: &[u64; 6] = &params[0..6].try_into().unwrap();

    // Perform the square root
    bls12_381_fp_sqrt(a, results);

    7
}

fn bls12_381_fp_sqrt(a: &[u64; 6], results: &mut [u64]) {
    let a_big = from_limbs_le(a);

    // Attempt to compute the square root of a
    let sqrt = a_big.modpow(&P_DIV_4, &P);

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

        results[1..7].copy_from_slice(&to_limbs_le::<6>(&sqrt_nqr));
        return;
    }

    results[1..7].copy_from_slice(&to_limbs_le::<6>(&sqrt));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bls12_381_fp_mul(a: &[u64; 6], b: &[u64; 6]) -> [u64; 6] {
        let a_big = from_limbs_le(a);
        let b_big = from_limbs_le(b);
        let ab_big = (a_big * b_big) % &*P;
        to_limbs_le::<6>(&ab_big)
    }

    #[test]
    fn test_sqrt_one() {
        let x = [1, 0, 0, 0, 0, 0];
        let expected_sqrt = [1, 0, 0, 0, 0, 0];

        let mut results = [0; 7];
        fcall_bls12_381_fp_sqrt(&x, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..7].try_into().unwrap();
        assert_eq!(has_sqrt, 1);
        assert_eq!(sqrt, &expected_sqrt);
        assert_eq!(bls12_381_fp_mul(sqrt, sqrt), x);
    }

    #[test]
    fn test_sqrt() {
        let x = [
            0xf22cb1516a067d13,
            0x3e46be206ab02de6,
            0x93153c30d0917c98,
            0x597d68ca77b5fa6d,
            0x44a50733df914e5e,
            0xf7377b1bb431d82,
        ];
        let expected_sqrt = [
            0x516e9b68ec7e4040,
            0x4b1f0de82104d372,
            0x7e742e30000909d7,
            0x44051766a1553492,
            0xe7043ea4bffc292f,
            0x3efcb69d6bf0ce0,
        ];

        let mut results = [0; 7];
        fcall_bls12_381_fp_sqrt(&x, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..7].try_into().unwrap();
        assert_eq!(has_sqrt, 1);
        assert_eq!(sqrt, &expected_sqrt);
        assert_eq!(bls12_381_fp_mul(sqrt, sqrt), x);
    }

    #[test]
    fn test_no_sqrt() {
        let x = [
            0x361799ccd540a764,
            0xf606e6b453a13bd8,
            0x8880bd6a4b0b963a,
            0x8c9a8b3ba67f6d02,
            0x922d30923791c733,
            0x1975e3ccd03944ca,
        ];
        let expected_sqrt = [
            0x5514d9e1a2faebf1,
            0x391ed94dec028013,
            0x5a8c79b17991fded,
            0x56207337f5f736d0,
            0xc6f1181533cc4b6,
            0xb1d40edb0c1fec0,
        ]; // sqrt(x * NQR)

        let mut results = [0; 7];
        fcall_bls12_381_fp_sqrt(&x, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..7].try_into().unwrap();
        assert_eq!(has_sqrt, 0);
        assert_eq!(sqrt, &expected_sqrt);
        let nqr = to_limbs_le(&NQR);
        assert_eq!(bls12_381_fp_mul(sqrt, sqrt), bls12_381_fp_mul(&x, &nqr));
    }
}
