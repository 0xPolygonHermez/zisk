//! Operations in the degree 12 extension Fp12 of the BN254 curve

use crate::{fcall_msb_pos_256, zisklib::lib::utils::eq};

use super::{
    constants::{
        FROBENIUS_GAMMA11, FROBENIUS_GAMMA12, FROBENIUS_GAMMA13, FROBENIUS_GAMMA14,
        FROBENIUS_GAMMA15, FROBENIUS_GAMMA21, FROBENIUS_GAMMA22, FROBENIUS_GAMMA23,
        FROBENIUS_GAMMA24, FROBENIUS_GAMMA25, FROBENIUS_GAMMA31, FROBENIUS_GAMMA32,
        FROBENIUS_GAMMA33, FROBENIUS_GAMMA34, FROBENIUS_GAMMA35,
    },
    fp2::{conjugate_fp2_bn254, mul_fp2_bn254, scalar_mul_fp2_bn254},
    fp6::{
        add_fp6_bn254, dbl_fp6_bn254, inv_fp6_bn254, mul_fp6_bn254, neg_fp6_bn254,
        sparse_mula_fp6_bn254, sparse_mulb_fp6_bn254, sparse_mulc_fp6_bn254, square_fp6_bn254,
        sub_fp6_bn254,
    },
};

/// Multiplication in the degree 12 extension of the BN254 curve
//
// in: (a1 + a2·w),(b1 + b2·w) ∈ Fp12, where ai,bi ∈ Fp6
// out: (a1 + a2·w)·(b1 + b2·w) = (c1 + c2·w) ∈ Fp12, where:
//      - c1 = a1·b1 + a2·b2·v
//      - c2 = (a1+a2)·(b1+b2) - a1·b1 - a2·b2
#[inline]
pub fn mul_fp12_bn254(a: &[u64; 48], b: &[u64; 48]) -> [u64; 48] {
    let a1 = &a[0..24].try_into().unwrap();
    let a2 = &a[24..48].try_into().unwrap();
    let b1 = &b[0..24].try_into().unwrap();
    let b2 = &b[24..48].try_into().unwrap();

    let a1b1 = mul_fp6_bn254(a1, b1);
    let a2b2 = mul_fp6_bn254(a2, b2);

    let a2b2v = sparse_mula_fp6_bn254(&a2b2, &[1, 0, 0, 0, 0, 0, 0, 0]);
    let c1 = add_fp6_bn254(&a1b1, &a2b2v);

    let a1_plus_a2 = add_fp6_bn254(a1, a2);
    let b1_plus_b2 = add_fp6_bn254(b1, b2);
    let mut c2 = mul_fp6_bn254(&a1_plus_a2, &b1_plus_b2);
    c2 = sub_fp6_bn254(&c2, &a1b1);
    c2 = sub_fp6_bn254(&c2, &a2b2);

    let mut result = [0; 48];
    result[0..24].copy_from_slice(&c1);
    result[24..48].copy_from_slice(&c2);
    result
}

/// Multiplication of a = a1 + a2·w and b = 1 + (b21 + b22·v)·w in the degree 12 extension of the BN254 curve
//
// in: (a1 + a2·w),(b1 + b2·w) ∈ Fp12, where ai ∈ Fp6, b1 = 1 and b2 = b21 + b22·v, with b21,b22 ∈ Fp2
// out: (a1 + a2·w)·(b1 + b2·w) = (c1 + c2·w) ∈ Fp12, where:
//      - c1 = a1 + a2·(b21·v + b22·v²)
//      - c2 = a2 + a1·(b21 + b22·v)
#[inline]
pub fn sparse_mul_fp12_bn254(a: &[u64; 48], b: &[u64; 16]) -> [u64; 48] {
    let a1 = &a[0..24].try_into().unwrap();
    let a2 = &a[24..48].try_into().unwrap();

    let mut c1 = sparse_mulc_fp6_bn254(&a2, b);
    c1 = add_fp6_bn254(&c1, a1);

    let mut c2 = sparse_mulb_fp6_bn254(a1, b);
    c2 = add_fp6_bn254(&c2, a2);

    let mut result = [0; 48];
    result[0..24].copy_from_slice(&c1);
    result[24..48].copy_from_slice(&c2);
    result
}

/// Squaring in the degree 12 extension of the BN254 curve
//
// in: (a1 + a2·w) ∈ Fp12, where ai ∈ Fp6
// out: (a1 + a2·w)² = (c1 + c2·w) ∈ Fp12, where:
//      - c1 = (a1-a2)·(a1-a2·v) + a1·a2 + a1·a2·v
//      - c2 = 2·a1·a2
#[inline]
pub fn square_fp12_bn254(a: &[u64; 48]) -> [u64; 48] {
    let a1 = &a[0..24].try_into().unwrap();
    let a2 = &a[24..48].try_into().unwrap();

    // a1·a2, a2·v, a1·a2·v
    let a1a2 = mul_fp6_bn254(a1, a2);
    let a2v = sparse_mula_fp6_bn254(a2, &[1, 0, 0, 0, 0, 0, 0, 0]);
    let a1a2v = sparse_mula_fp6_bn254(&a1a2, &[1, 0, 0, 0, 0, 0, 0, 0]);

    // c1
    let a1_minus_a2 = sub_fp6_bn254(a1, a2);
    let a1_minus_a2v = sub_fp6_bn254(a1, &a2v);
    let mut c1 = mul_fp6_bn254(&a1_minus_a2, &a1_minus_a2v);
    c1 = add_fp6_bn254(&c1, &a1a2);
    c1 = add_fp6_bn254(&c1, &a1a2v);

    // c2
    let c2 = dbl_fp6_bn254(&a1a2);

    let mut result = [0; 48];
    result[0..24].copy_from_slice(&c1);
    result[24..48].copy_from_slice(&c2);
    result
}

/// Inversion in the degree 12 extension of the BN254 curve
//
// in: (a1 + a2·w) ∈ Fp12, where ai ∈ Fp6
// out: (a1 + a2·w)⁻¹ = (c1 + c2·w) ∈ Fp12, where:
//      - c1 = a1·(a1² - a2²·v)⁻¹
//      - c2 = -a2·(a1² - a2²·v)⁻¹
#[inline]
pub fn inv_fp12_bn254(a: &[u64; 48]) -> [u64; 48] {
    let a1 = &a[0..24].try_into().unwrap();
    let a2 = &a[24..48].try_into().unwrap();

    let a1_sq = square_fp6_bn254(a1);
    let a2_sq = square_fp6_bn254(a2);

    let a2_sqv = sparse_mula_fp6_bn254(&a2_sq, &[1, 0, 0, 0, 0, 0, 0, 0]);
    let a1_sq_minus_a2_sqv = sub_fp6_bn254(&a1_sq, &a2_sqv);
    let inv = inv_fp6_bn254(&a1_sq_minus_a2_sqv);

    let c1 = mul_fp6_bn254(a1, &inv);
    let c2 = neg_fp6_bn254(&mul_fp6_bn254(a2, &inv));

    let mut result = [0; 48];
    result[0..24].copy_from_slice(&c1);
    result[24..48].copy_from_slice(&c2);
    result
}

/// Conjugation in the degree 12 extension of the BN254 curve
#[inline]
pub fn conjugate_fp12_bn254(a: &[u64; 48]) -> [u64; 48] {
    let mut result = [0; 48];
    result[0..24].copy_from_slice(&a[0..24]);
    result[24..48].copy_from_slice(&neg_fp6_bn254(&a[24..48].try_into().unwrap()));
    result
}

/// First Frobenius operator in the degree 12 extension of the BN254 curve
//
// in: (a1 + a2·w) = ((a11 + a12v + a13v²) + (a21 + a22v + a23v²)·w) ∈ Fp12, where ai ∈ Fp6 and aij ∈ Fp2
// out: (a1 + a2·w)ᵖ = (c1 + c2·w) ∈ Fp12, where:
//      - c1 = a̅11     + a̅12·γ12·v + a̅13·γ14·v²
//      - c2 = a̅21·γ11 + a̅22·γ13·v + a̅23·γ15·v²
#[inline]
pub fn frobenius1_fp12_bn254(a: &[u64; 48]) -> [u64; 48] {
    let a11 = &a[0..8].try_into().unwrap();
    let a12 = &a[8..16].try_into().unwrap();
    let a13 = &a[16..24].try_into().unwrap();
    let a21 = &a[24..32].try_into().unwrap();
    let a22 = &a[32..40].try_into().unwrap();
    let a23 = &a[40..48].try_into().unwrap();

    let mut result = [0; 48];

    // c1 = a̅11 + a̅12·γ12·v + a̅13·γ14·v²
    result[0..8].copy_from_slice(&conjugate_fp2_bn254(a11));
    let mut tmp = conjugate_fp2_bn254(a12);
    result[8..16].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA12));
    tmp = conjugate_fp2_bn254(a13);
    result[16..24].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA14));

    // c2 = a̅21·γ11 + a̅22·γ13·v + a̅23·γ15·v²
    tmp = conjugate_fp2_bn254(a21);
    result[24..32].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA11));
    tmp = conjugate_fp2_bn254(a22);
    result[32..40].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA13));
    tmp = conjugate_fp2_bn254(a23);
    result[40..48].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA15));

    result
}

/// Second Frobenius operator in the degree 12 extension of the BN254 curve
//
// in: (a1 + a2·w) = ((a11 + a12v + a13v²) + (a21 + a22v + a23v²)) ∈ Fp12, where ai ∈ Fp6 and aij ∈ Fp2
// out: (a1 + a2·w)ᵖ˙ᵖ = (c1 + c2·w) ∈ Fp12, where:
//      - c1 = a11     + a12·γ22·v + a13·γ24·v²
//      - c2 = a21·γ21 + a22·γ23·v + a23·γ25·v²
#[inline]
pub fn frobenius2_fp12_bn254(a: &[u64; 48]) -> [u64; 48] {
    let a11: &[u64; 8] = &a[0..8].try_into().unwrap();
    let a12 = &a[8..16].try_into().unwrap();
    let a13 = &a[16..24].try_into().unwrap();
    let a21 = &a[24..32].try_into().unwrap();
    let a22 = &a[32..40].try_into().unwrap();
    let a23 = &a[40..48].try_into().unwrap();

    let mut result = [0; 48];

    // c1 = a11 + a12·γ22·v + a13·γ24·v²
    result[0..8].copy_from_slice(a11);
    result[8..16].copy_from_slice(&scalar_mul_fp2_bn254(a12, &FROBENIUS_GAMMA22));
    result[16..24].copy_from_slice(&scalar_mul_fp2_bn254(a13, &FROBENIUS_GAMMA24));

    // c2 = a21·γ21 + a22·γ23·v + a23·γ25·v²
    result[24..32].copy_from_slice(&scalar_mul_fp2_bn254(a21, &FROBENIUS_GAMMA21));
    result[32..40].copy_from_slice(&scalar_mul_fp2_bn254(a22, &FROBENIUS_GAMMA23));
    result[40..48].copy_from_slice(&scalar_mul_fp2_bn254(a23, &FROBENIUS_GAMMA25));

    result
}

/// Third Frobenius operator in the degree 12 extension of the BN254 curve
//
// in: (a1 + a2·w) = ((a11 + a12v + a13v²) + (a21 + a22v + a23v²)) ∈ Fp12, where ai ∈ Fp6 and aij ∈ Fp2
// out: (a1 + a2·w)ᵖ˙ᵖ˙ᵖ = (c1 + c2·w) ∈ Fp12, where:
//      - c1 = a̅11     + a̅12·γ32·v + a̅13·γ34·v²
//      - c2 = a̅21·γ31 + a̅22·γ33·v + a̅23·γ35·v²
#[inline]
pub fn frobenius3_fp12_bn254(a: &[u64; 48]) -> [u64; 48] {
    let a11 = &a[0..8].try_into().unwrap();
    let a12 = &a[8..16].try_into().unwrap();
    let a13 = &a[16..24].try_into().unwrap();
    let a21 = &a[24..32].try_into().unwrap();
    let a22 = &a[32..40].try_into().unwrap();
    let a23 = &a[40..48].try_into().unwrap();

    let mut result = [0; 48];

    // c1 = a̅11 + a̅12·γ32·v + a̅13·γ34·v²
    result[0..8].copy_from_slice(&conjugate_fp2_bn254(a11));
    let mut tmp = conjugate_fp2_bn254(a12);
    result[8..16].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA32));
    tmp = conjugate_fp2_bn254(a13);
    result[16..24].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA34));

    // c2 = a̅21·γ31 + a̅22·γ33·v + a̅23·γ35·v²
    tmp = conjugate_fp2_bn254(a21);
    result[24..32].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA31));
    tmp = conjugate_fp2_bn254(a22);
    result[32..40].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA33));
    tmp = conjugate_fp2_bn254(a23);
    result[40..48].copy_from_slice(&mul_fp2_bn254(&tmp, &FROBENIUS_GAMMA35));

    result
}

/// Exponentiation in the degree 12 extension of the BN254 curve
//
// in: e, (a1 + a2·w) ∈ Fp12, where e ∈ [0,p¹²-2] ai ∈ Fp6
// out: (c1 + c2·w) = (a1 + a2·w)^e ∈ Fp12
#[inline]
pub fn exp_fp12_bn254(e: u64, a: &[u64; 48]) -> [u64; 48] {
    let mut one = [0; 48];
    one[0] = 1;
    if eq(a, &[0; 48]) {
        return [0; 48];
    } else if eq(a, &one) {
        return one;
    }

    if e == 0 {
        return one;
    } else if e == 1 {
        return a.clone();
    }

    let (_, max_bit) = fcall_msb_pos_256(&[e, 0, 0, 0], &[0, 0, 0, 0]);

    // Perform the loop, based on the binary representation of e

    // We do the first iteration separately
    let e_bit = (e >> max_bit) & 1;
    assert_eq!(e_bit, 1); // the first received bit should be 1

    // Start the loop at a
    let mut result = a.clone();
    let mut e_rec = 1 << max_bit;

    // Perform the rest of the loop
    let _max_bit = max_bit as usize;
    for i in (0.._max_bit).rev() {
        // Always square
        result = square_fp12_bn254(&result);

        // Get the next bit b of e
        // If b == 1, we should multiply it by a, otherwise start the next iteration
        if ((e >> i) & 1) == 1 {
            result = mul_fp12_bn254(&result, a);

            // Reconstruct e
            e_rec |= 1 << i;
        }
    }

    // Check that the reconstructed e is equal to the input e
    assert_eq!(e_rec, e);

    result
}
