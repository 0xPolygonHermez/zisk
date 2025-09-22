//! Finite field Fp12 operations for BLS12-381

use crate::{fcall_msb_pos_384, zisklib::lib::utils::eq};

use super::{
    constants::{
        EXT_U, EXT_V, FROBENIUS_GAMMA11, FROBENIUS_GAMMA12, FROBENIUS_GAMMA13, FROBENIUS_GAMMA14,
        FROBENIUS_GAMMA15, FROBENIUS_GAMMA21, FROBENIUS_GAMMA22, FROBENIUS_GAMMA23,
        FROBENIUS_GAMMA24, FROBENIUS_GAMMA25,
    },
    fp2::{conjugate_fp2_bls12_381, mul_fp2_bls12_381, scalar_mul_fp2_bls12_381},
    fp6::{
        add_fp6_bls12_381, dbl_fp6_bls12_381, inv_fp6_bls12_381, mul_fp6_bls12_381,
        neg_fp6_bls12_381, sparse_mula_fp6_bls12_381, sparse_mulb_fp6_bls12_381,
        sparse_mulc_fp6_bls12_381, square_fp6_bls12_381, sub_fp6_bls12_381,
    },
};

/// Multiplication in Fp12
//
//  in: (a1 + a2·w),(b1 + b2·w) ∈ Fp12, where ai,bi ∈ Fp6
//  out: (a1 + a2·w)·(b1 + b2·w) = (c1 + c2·w) ∈ Fp12, where:
//       - c1 = a1·b1 + a2·b2·v
//       - c2 = (a1+a2)·(b1+b2) - a1·b1 - a2·b2
#[inline]
pub fn mul_fp12_bls12_381(a: &[u64; 72], b: &[u64; 72]) -> [u64; 72] {
    let a1 = &a[0..36].try_into().unwrap();
    let a2 = &a[36..72].try_into().unwrap();
    let b1 = &b[0..36].try_into().unwrap();
    let b2 = &b[36..72].try_into().unwrap();

    // a1·b1, a2·b2
    let a1_b1 = mul_fp6_bls12_381(a1, b1);
    let a2_b2 = mul_fp6_bls12_381(a2, b2);

    // c1 = a1·b1 + a2·b2·v
    let mut c1 = sparse_mula_fp6_bls12_381(&a2_b2, &EXT_V);
    c1 = add_fp6_bls12_381(&c1, &a1_b1);

    // c2 = (a1+a2)·(b1+b2) - a1·b1 - a2·b2
    let a1_plus_a2 = add_fp6_bls12_381(a1, a2);
    let b1_plus_b2 = add_fp6_bls12_381(b1, b2);
    let mut c2 = mul_fp6_bls12_381(&a1_plus_a2, &b1_plus_b2);
    c2 = sub_fp6_bls12_381(&c2, &a1_b1);
    c2 = sub_fp6_bls12_381(&c2, &a2_b2);

    [c1, c2].concat().try_into().unwrap()
}

/// Multiplication of a = a1 + a2·w and b = 1 + (b22·v + b23·v²)·w in Fp12
//
//  in: (a1 + a2·w),(b1 + b2·w) ∈ Fp12, where ai ∈ Fp6, b1 = 1 and b2 = b22·v + b23·v², with b22,b23 ∈ Fp2
//  out: (a1 + a2·w)·(b1 + b2·w) = (c1 + c2·w) ∈ Fp12, where:
//       - c1 = a1 + a2·(b23·(1+u) + b22·v²)
//       - c2 = a2 + a1·(b22·v + b23·v²)
#[inline]
pub fn sparse_mul_fp12_bls12_381(a: &[u64; 72], b: &[u64; 24]) -> [u64; 72] {
    let a1 = &a[0..36].try_into().unwrap();
    let a2 = &a[36..72].try_into().unwrap();
    let b22 = &b[0..12].try_into().unwrap();
    let b23 = &b[12..24].try_into().unwrap();

    // c1 = a1 + a2·(b23·(1+u) + b22·v²)
    let b23u = mul_fp2_bls12_381(&EXT_U, b23);
    let mut c1 = sparse_mulc_fp6_bls12_381(a2, &[b23u, *b22].concat().try_into().unwrap());
    c1 = add_fp6_bls12_381(&c1, a1);

    // c2 = a2 + a1·(b22·v + b23·v²)
    let mut c2 = sparse_mulb_fp6_bls12_381(a1, &[*b22, *b23].concat().try_into().unwrap());
    c2 = add_fp6_bls12_381(&c2, a2);

    [c1, c2].concat().try_into().unwrap()
}

/// Squaring in Fp12
//
//  in: (a1 + a2·w) ∈ Fp12, where ai ∈ Fp6
//  out: (a1 + a2·w)² = (c1 + c2·w) ∈ Fp12, where:
//       - c1 = (a1-a2)·(a1-a2·v) + a1·a2 + a1·a2·v
//       - c2 = 2·a1·a2
#[inline]
pub fn square_fp12_bls12_381(a: &[u64; 72]) -> [u64; 72] {
    let a1 = &a[0..36].try_into().unwrap();
    let a2 = &a[36..72].try_into().unwrap();

    // a1·a2, a2·v, a1·a2·v
    let a1_a2 = mul_fp6_bls12_381(a1, a2);
    let a2_v = sparse_mula_fp6_bls12_381(a2, &EXT_V);
    let a1_a2_v = sparse_mula_fp6_bls12_381(&a1_a2, &EXT_V);

    // c2 = 2·a1·a2
    let c2 = dbl_fp6_bls12_381(&a1_a2);

    // c1 = (a1-a2)·(a1-a2·v) + a1·a2 + a1·a2·v
    let a1_minus_a2 = sub_fp6_bls12_381(a1, a2);
    let a1_minus_a2v = sub_fp6_bls12_381(a1, &a2_v);
    let mut c1 = mul_fp6_bls12_381(&a1_minus_a2, &a1_minus_a2v);
    c1 = add_fp6_bls12_381(&c1, &a1_a2);
    c1 = add_fp6_bls12_381(&c1, &a1_a2_v);

    [c1, c2].concat().try_into().unwrap()
}

/// Inversion in Fp12
//
//  in: (a1 + a2·w) ∈ Fp12, where ai ∈ Fp6
//  out: (a1 + a2·w)⁻¹ = (c1 + c2·w) ∈ Fp12, where:
//       - c1 = a1·(a1² - a2²·v)⁻¹
//       - c2 = -a2·(a1² - a2²·v)⁻¹
#[inline]
pub fn inv_fp12_bls12_381(a: &[u64; 72]) -> [u64; 72] {
    let a1 = &a[0..36].try_into().unwrap();
    let a2 = &a[36..72].try_into().unwrap();

    // a1², a2², a2²·v
    let a1_square = square_fp6_bls12_381(a1);
    let a2_square = square_fp6_bls12_381(a2);
    let a2_square_v = sparse_mula_fp6_bls12_381(&a2_square, &EXT_V);

    // (a1² - a2²·v)⁻¹
    let mut denom = sub_fp6_bls12_381(&a1_square, &a2_square_v);
    denom = inv_fp6_bls12_381(&denom);

    // c1 = a1·(a1² - a2²·v)⁻¹, c2 = -a2·(a1² - a2²·v)⁻¹
    let c1 = mul_fp6_bls12_381(a1, &denom);
    let c2 = neg_fp6_bls12_381(&mul_fp6_bls12_381(a2, &denom));

    [c1, c2].concat().try_into().unwrap()
}

/// Conjugation in Fp12
#[inline]
pub fn conjugate_fp12_bls12_381(a: &[u64; 72]) -> [u64; 72] {
    let mut result = [0; 72];
    result[0..36].copy_from_slice(&a[0..36]);
    result[36..72].copy_from_slice(&neg_fp6_bls12_381(&a[36..72].try_into().unwrap()));
    result
}

/// First Frobenius operator in Fp12
//
//  in: (a1 + a2·w) = ((a11 + a12v + a13v²) + (a21 + a22v + a23v²)·w) ∈ Fp12, where ai ∈ Fp6 and aij ∈ Fp2
//  out: (a1 + a2·w)ᵖ = (c1 + c2·w) ∈ Fp12, where:
//       - c1 = a̅11     + a̅12·γ12·v + a̅13·γ14·v²
//       - c2 = a̅21·γ11 + a̅22·γ13·v + a̅23·γ15·v²
#[inline]
pub fn frobenius1_fp12_bls12_381(a: &[u64; 72]) -> [u64; 72] {
    let a11 = &a[0..12].try_into().unwrap();
    let a12 = &a[12..24].try_into().unwrap();
    let a13 = &a[24..36].try_into().unwrap();
    let a21 = &a[36..48].try_into().unwrap();
    let a22 = &a[48..60].try_into().unwrap();
    let a23 = &a[60..72].try_into().unwrap();

    let mut result = [0; 72];

    // c1 = a̅11 + a̅12·γ12·v + a̅13·γ14·v²
    result[0..12].copy_from_slice(&conjugate_fp2_bls12_381(a11));
    let mut tmp = conjugate_fp2_bls12_381(a12);
    result[12..24].copy_from_slice(&mul_fp2_bls12_381(&tmp, &FROBENIUS_GAMMA12));
    tmp = conjugate_fp2_bls12_381(a13);
    result[24..36].copy_from_slice(&scalar_mul_fp2_bls12_381(&tmp, &FROBENIUS_GAMMA14));

    // c2 = a̅21·γ11 + a̅22·γ13·v + a̅23·γ15·v²
    tmp = conjugate_fp2_bls12_381(a21);
    result[36..48].copy_from_slice(&mul_fp2_bls12_381(&tmp, &FROBENIUS_GAMMA11));
    tmp = conjugate_fp2_bls12_381(a22);
    result[48..60].copy_from_slice(&mul_fp2_bls12_381(&tmp, &FROBENIUS_GAMMA13));
    tmp = conjugate_fp2_bls12_381(a23);
    result[60..72].copy_from_slice(&mul_fp2_bls12_381(&tmp, &FROBENIUS_GAMMA15));

    result
}

/// Second Frobenius operator in Fp12
//
//  in: (a1 + a2·w) = ((a11 + a12v + a13v²) + (a21 + a22v + a23v²)·w) ∈ Fp12, where ai ∈ Fp6 and aij ∈ Fp2
//  out: (a1 + a2·w)ᵖ˙ᵖ = (c1 + c2·w) ∈ Fp12, where:
//       - c1 = a11     + a12·γ22·v + a13·γ24·v²
//       - c2 = a21·γ21 + a22·γ23·v + a23·γ25·v²
#[inline]
pub fn frobenius2_fp12_bls12_381(a: &[u64; 72]) -> [u64; 72] {
    let a11: &[u64; 12] = &a[0..12].try_into().unwrap();
    let a12 = &a[12..24].try_into().unwrap();
    let a13 = &a[24..36].try_into().unwrap();
    let a21 = &a[36..48].try_into().unwrap();
    let a22 = &a[48..60].try_into().unwrap();
    let a23 = &a[60..72].try_into().unwrap();

    let mut result = [0; 72];

    // c1 = a11 + a12·γ22·v + a13·γ24·v²
    result[0..12].copy_from_slice(a11);
    result[12..24].copy_from_slice(&scalar_mul_fp2_bls12_381(a12, &FROBENIUS_GAMMA22));
    result[24..36].copy_from_slice(&scalar_mul_fp2_bls12_381(a13, &FROBENIUS_GAMMA24));

    // c2 = a21·γ21 + a22·γ23·v + a23·γ25·v²
    result[36..48].copy_from_slice(&scalar_mul_fp2_bls12_381(a21, &FROBENIUS_GAMMA21));
    result[48..60].copy_from_slice(&scalar_mul_fp2_bls12_381(a22, &FROBENIUS_GAMMA23));
    result[60..72].copy_from_slice(&scalar_mul_fp2_bls12_381(a23, &FROBENIUS_GAMMA25));

    result
}

/// Exponentiation in Fp12
//
// in: e, (a1 + a2·w) ∈ Fp12, where e ∈ [0,p¹²-2] ai ∈ Fp6
// out: (c1 + c2·w) = (a1 + a2·w)^e ∈ Fp12
#[inline]
pub fn exp_fp12_bls12_381(e: u64, a: &[u64; 72]) -> [u64; 72] {
    let one = {
        let mut tmp = [0; 72];
        tmp[0] = 1;
        tmp
    };

    if eq(a, &[0; 72]) {
        return [0; 72];
    } else if eq(a, &one) {
        return one;
    }

    if e == 0 {
        return one;
    } else if e == 1 {
        return *a;
    }

    let (_, max_bit) = fcall_msb_pos_384(&[e, 0, 0, 0, 0, 0], &[0, 0, 0, 0, 0, 0]);

    // Perform the loop, based on the binary representation of e

    // We do the first iteration separately
    let e_bit = (e >> max_bit) & 1;
    assert_eq!(e_bit, 1); // the first received bit should be 1

    // Start the loop at a
    let mut result = *a;
    let mut e_rec = 1 << max_bit;

    // Perform the rest of the loop
    let _max_bit = max_bit as usize;
    for i in (0.._max_bit).rev() {
        // Always square
        result = square_fp12_bls12_381(&result);

        // Get the next bit b of e
        // If b == 1, we should multiply it by a, otherwise start the next iteration
        if ((e >> i) & 1) == 1 {
            result = mul_fp12_bls12_381(&result, a);

            // Reconstruct e
            e_rec |= 1 << i;
        }
    }

    // Check that the reconstructed e is equal to the input e
    assert_eq!(e_rec, e);

    result
}
