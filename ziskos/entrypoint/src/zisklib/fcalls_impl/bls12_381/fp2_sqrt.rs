use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::{One, Zero};

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, u64_digits_from_biguint};

use super::{
    fp2_inv::{bls12_381_fp2_mul, bls12_381_fp2_square},
    fp_inv::{bls12_381_fp_add, bls12_381_fp_neg},
    I, NQR_FP2, ONE, P_MINUS_1_DIV_2, P_MINUS_3_DIV_4, P_MINUS_ONE,
};

/// Computes the square root of a non-zero field element in Fp2
pub fn fcall_bls12_381_fp2_sqrt(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a: &[u64; 12] = &params[0..12].try_into().unwrap();

    // Perform the square root
    let _results = bls12_381_fp2_sqrt_13(a);
    results.copy_from_slice(&_results);

    13
}

pub fn bls12_381_fp2_sqrt_13(a: &[u64; 12]) -> [u64; 13] {
    let mut results = [0u64; 13];

    // Perform the square root
    let (sqrt, is_qr) = bls12_381_fp2_sqrt(a);
    results[0] = is_qr as u64;
    if !is_qr {
        // To check that a is indeed a non-quadratic residue, we check that
        // a * NQR is a quadratic residue for some fixed known non-quadratic residue NQR
        let a_nqr = bls12_381_fp2_mul(a, &NQR_FP2);

        // Compute the square root of a * NQR
        let sqrt_nqr = bls12_381_fp2_sqrt(&a_nqr).0;

        results[1..13].copy_from_slice(&sqrt_nqr);
    } else {
        results[1..13].copy_from_slice(&sqrt);
    }
    results
}

/// Algorithm 9 from https://eprint.iacr.org/2012/685.pdf
/// Square root computation over F_p^2, with p ≡ 3 (mod 4)
fn bls12_381_fp2_sqrt(a: &[u64; 12]) -> ([u64; 12], bool) {
    // Step 1: a1 ← a^((p-3)/4)
    let a1 = bls12_381_fp2_exp(a, &P_MINUS_3_DIV_4);

    // Step 2: α ← a1 * a1 * a
    let a1_a = bls12_381_fp2_mul(&a1, a);
    let alpha = bls12_381_fp2_mul(&a1, &a1_a);

    // Step 3: a0 ← α^p * α = conjugate(α) * α
    let a0 = bls12_381_fp2_mul(&bls12_381_fp2_conjugate(&alpha), &alpha);

    // Step 4-6: if a0 == -1 then return false (no square root)
    if a0 == P_MINUS_ONE {
        return ([0u64; 12], false);
    }

    // Step 7: x0 ← a1 * a
    let x0 = a1_a;

    // Step 8-13: compute x based on α
    let x = if alpha == P_MINUS_ONE {
        // Step 9: x ← i * x0
        bls12_381_fp2_mul(&I, &x0)
    } else {
        // Step 11: b ← (1 + α)^((p-1)/2)
        let one_plus_alpha = bls12_381_fp2_add(&ONE, &alpha);
        let b = bls12_381_fp2_exp(&one_plus_alpha, &P_MINUS_1_DIV_2);

        // Step 12: x ← b * x0
        bls12_381_fp2_mul(&b, &x0)
    };

    (x, true)
}

pub(crate) fn bls12_381_fp2_conjugate(a: &[u64; 12]) -> [u64; 12] {
    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&a[0..6]);
    let imaginary_part: &[u64; 6] = &a[6..12].try_into().unwrap();
    let neg_imaginary_part = bls12_381_fp_neg(imaginary_part);
    result[6..12].copy_from_slice(&neg_imaginary_part);
    result
}

pub(crate) fn bls12_381_fp2_add(a: &[u64; 12], b: &[u64; 12]) -> [u64; 12] {
    let a_real = &a[0..6].try_into().unwrap();
    let a_imaginary = &a[6..12].try_into().unwrap();
    let b_real = &b[0..6].try_into().unwrap();
    let b_imaginary = &b[6..12].try_into().unwrap();

    let real_part = bls12_381_fp_add(a_real, b_real);
    let imaginary_part = bls12_381_fp_add(a_imaginary, b_imaginary);

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&real_part);
    result[6..12].copy_from_slice(&imaginary_part);
    result
}

pub(crate) fn bls12_381_fp2_exp(a: &[u64; 12], e: &BigUint) -> [u64; 12] {
    let mut result = [0u64; 12];
    result[0] = 1;

    let mut base = *a;
    let mut exp = e.clone();

    while !exp.is_zero() {
        if (&exp & BigUint::one()) == BigUint::one() {
            result = bls12_381_fp2_mul(&result, &base);
        }
        base = bls12_381_fp2_mul(&base, &base);
        exp >>= 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt_one() {
        let x = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let expected_sqrt = P_MINUS_ONE;

        let mut results = [0; 13];
        fcall_bls12_381_fp2_sqrt(&x, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..13].try_into().unwrap();
        assert_eq!(has_sqrt, 1);
        assert_eq!(sqrt, &expected_sqrt);
        assert_eq!(bls12_381_fp2_mul(sqrt, sqrt), x);
    }

    #[test]
    fn test_sqrt() {
        let x = [
            0x10486089be1876e9,
            0xcf0c3012bf0c13ef,
            0x51621421d2c37a8d,
            0xd52db71259449a47,
            0x370fd7a0a4be29da,
            0xc3d4fd75c076215,
            0x3e6ff1a3151b0959,
            0x9f0b2a8dea2c9f82,
            0xb83d47ccb71501e2,
            0xa8c917818d857f05,
            0xc48150d1cd95e0c6,
            0x112ca78116187cc8,
        ];
        let expected_sqrt = [
            0xcca66dfc0d7f69c9,
            0xaf22cf40d2f4555,
            0x92a6870798aff4d7,
            0xe595438fb87ee1fc,
            0x6f5e96c633b39798,
            0x215675032da3de5,
            0x1ef8b538e151e6f3,
            0x94b37a0021182ef6,
            0xea0d1db797288ba2,
            0x567c72d5af34be56,
            0x5470d2ed597db716,
            0x10b61243878d0170,
        ];

        let mut results = [0; 13];
        fcall_bls12_381_fp2_sqrt(&x, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..13].try_into().unwrap();
        assert_eq!(has_sqrt, 1);
        assert_eq!(sqrt, &expected_sqrt);
        assert_eq!(bls12_381_fp2_mul(sqrt, sqrt), x);
    }

    #[test]
    fn test_no_sqrt() {
        let x = [
            0x5531f66e0c366bf8,
            0x35f8f154ff2974e6,
            0xaa81eb7e92ae7b5e,
            0x8a521c9ff4654bc0,
            0xa224f0e84356bba8,
            0xffbbc4bdd5425cb,
            0xf16972261c97a569,
            0xbf071b2a52d05a68,
            0xbaa99b2bc5260f74,
            0xedbd0c20e26eb5e5,
            0x6f3229e291d1d67a,
            0x119353ab08784f06,
        ];
        let expected_sqrt = [
            0x6d8e1fc1edb82644,
            0xa6964afc770dab5d,
            0x37d90a0e925a572d,
            0x3547fbc3f051b409,
            0xd3cdef010df23067,
            0x159b8fd2cca0a180,
            0xe0c163a5a7441092,
            0xf61c7202d7c3af80,
            0xf80c7aa929cb1e62,
            0xa076467c356a64cf,
            0x695e3d70b6a86704,
            0xb1ecd8ecdb0e8d2,
        ]; // sqrt(x * NQR)

        let mut results = [0; 13];
        fcall_bls12_381_fp2_sqrt(&x, &mut results);
        let has_sqrt = results[0];
        let sqrt = &results[1..13].try_into().unwrap();
        assert_eq!(has_sqrt, 0);
        assert_eq!(sqrt, &expected_sqrt);
        assert_eq!(bls12_381_fp2_mul(sqrt, sqrt), bls12_381_fp2_mul(&x, &NQR_FP2));
    }
}
