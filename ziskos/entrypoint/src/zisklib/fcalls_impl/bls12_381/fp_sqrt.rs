use lazy_static::lazy_static;
use num_bigint::BigUint;

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};

use super::{NQR_FP, P, P_DIV_4};

/// Computes the square root of a non-zero field element in Fp
pub fn fcall_bls12_381_fp_sqrt(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a: &[u64; 6] = &params[0..6].try_into().unwrap();

    // Perform the square root
    bls12_381_fp_sqrt(a, results);

    7
}

pub fn bls12_381_fp_sqrt(a: &[u64; 6], results: &mut [u64]) {
    let a_big = biguint_from_u64_digits(a);

    // Attempt to compute the square root of a
    let sqrt = a_big.modpow(&P_DIV_4, &P);

    // Check if a is a quadratic residue
    let square = (&sqrt * &sqrt) % &*P;
    let a_is_qr = square == a_big;
    results[0] = a_is_qr as u64;
    if !a_is_qr {
        // To check that a is indeed a non-quadratic residue, we check that
        // a * NQR is a quadratic residue for some fixed known non-quadratic residue NQR
        let a_nqr = (a_big * &*NQR_FP) % &*P;

        // Compute the square root of a * NQR
        let sqrt_nqr = a_nqr.modpow(&P_DIV_4, &P);

        results[1..7].copy_from_slice(&n_u64_digits_from_biguint::<6>(&sqrt_nqr));
        return;
    }

    results[1..7].copy_from_slice(&n_u64_digits_from_biguint::<6>(&sqrt));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bls12_381_fp_mul(a: &[u64; 6], b: &[u64; 6]) -> [u64; 6] {
        let a_big = biguint_from_u64_digits(a);
        let b_big = biguint_from_u64_digits(b);
        let ab_big = (a_big * b_big) % &*P;
        n_u64_digits_from_biguint::<6>(&ab_big)
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
        let nqr = n_u64_digits_from_biguint::<6>(&NQR_FP);
        assert_eq!(bls12_381_fp_mul(sqrt, sqrt), bls12_381_fp_mul(&x, &nqr));
    }
}
