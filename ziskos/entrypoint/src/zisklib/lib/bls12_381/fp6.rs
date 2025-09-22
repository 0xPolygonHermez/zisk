//! Finite field Fp6 operations for BLS12-381

use super::{
    constants::EXT_U,
    fp2::{
        add_fp2_bls12_381, dbl_fp2_bls12_381, inv_fp2_bls12_381, mul_fp2_bls12_381,
        neg_fp2_bls12_381, square_fp2_bls12_381, sub_fp2_bls12_381,
    },
};

/// Addition in Fp6
#[inline]
pub fn add_fp6_bls12_381(a: &[u64; 36], b: &[u64; 36]) -> [u64; 36] {
    let mut result = [0; 36];
    for i in 0..3 {
        let a_i = &a[i * 12..(i + 1) * 12].try_into().unwrap();
        let b_i = &b[i * 12..(i + 1) * 12].try_into().unwrap();
        let c_i = add_fp2_bls12_381(a_i, b_i);
        result[i * 12..(i + 1) * 12].copy_from_slice(&c_i);
    }
    result
}

/// Doubling in Fp6
#[inline]
pub fn dbl_fp6_bls12_381(a: &[u64; 36]) -> [u64; 36] {
    let mut result = [0; 36];
    for i in 0..3 {
        let a_i = &a[i * 12..(i + 1) * 12].try_into().unwrap();
        let c_i = dbl_fp2_bls12_381(a_i);
        result[i * 12..(i + 1) * 12].copy_from_slice(&c_i);
    }
    result
}

/// Negation in Fp6
#[inline]
pub fn neg_fp6_bls12_381(a: &[u64; 36]) -> [u64; 36] {
    let mut result = [0; 36];
    for i in 0..3 {
        let a_i = &a[i * 12..(i + 1) * 12].try_into().unwrap();
        let c_i = neg_fp2_bls12_381(a_i);
        result[i * 12..(i + 1) * 12].copy_from_slice(&c_i);
    }
    result
}

/// Subtraction in Fp6
#[inline]
pub fn sub_fp6_bls12_381(a: &[u64; 36], b: &[u64; 36]) -> [u64; 36] {
    let mut result = [0; 36];
    for i in 0..3 {
        let a_i = &a[i * 12..(i + 1) * 12].try_into().unwrap();
        let b_i = &b[i * 12..(i + 1) * 12].try_into().unwrap();
        let c_i = sub_fp2_bls12_381(a_i, b_i);
        result[i * 12..(i + 1) * 12].copy_from_slice(&c_i);
    }
    result
}

/// Multiplication in Fp6
//  in: (a1 + a2·v + a3·v²),(b1 + b2·v + b3·v²) ∈ Fp6, where ai,bi ∈ Fp2
//  out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//       - c1 = a1·b1 + [a2·b3 + a3·b2]·(1+u)
//       - c2 = a1·b2 + a2·b1 + (a3·b3)·(1+u)
//       - c3 = a1·b3 + a2·b2 + a3·b1
#[inline]
pub fn mul_fp6_bls12_381(a: &[u64; 36], b: &[u64; 36]) -> [u64; 36] {
    let a1 = &a[0..12].try_into().unwrap();
    let a2 = &a[12..24].try_into().unwrap();
    let a3 = &a[24..36].try_into().unwrap();
    let b1 = &b[0..12].try_into().unwrap();
    let b2 = &b[12..24].try_into().unwrap();
    let b3 = &b[24..36].try_into().unwrap();

    // c1 = a1·b1 + [a2·b3 + a3·b2]·(1+u)
    let mut c1 = mul_fp2_bls12_381(a2, b3);
    c1 = add_fp2_bls12_381(&c1, &mul_fp2_bls12_381(a3, b2));
    c1 = mul_fp2_bls12_381(&c1, &EXT_U);
    c1 = add_fp2_bls12_381(&c1, &mul_fp2_bls12_381(a1, b1));

    // c2 = a1·b2 + a2·b1 + (a3·b3)·(1+u)
    let mut c2 = mul_fp2_bls12_381(a3, b3);
    c2 = mul_fp2_bls12_381(&c2, &EXT_U);
    c2 = add_fp2_bls12_381(&c2, &mul_fp2_bls12_381(a1, b2));
    c2 = add_fp2_bls12_381(&c2, &mul_fp2_bls12_381(a2, b1));

    // c3 = a1·b3 + a2·b2 + a3·b1
    let mut c3 = mul_fp2_bls12_381(a1, b3);
    c3 = add_fp2_bls12_381(&c3, &mul_fp2_bls12_381(a2, b2));
    c3 = add_fp2_bls12_381(&c3, &mul_fp2_bls12_381(a3, b1));

    [c1, c2, c3].concat().try_into().unwrap()
}
/// Multiplication of a = a1 + a2·v + a3·v² and b = b2·v in Fp6
//
//  in: (a1 + a2·v + a3·v²),(b2·v) ∈ Fp6, where ai,bi ∈ Fp2
//  out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//       - c1 = a3·b2·(1+u)
//       - c2 = a1·b2
//       - c3 = a2·b2
#[inline]
pub fn sparse_mula_fp6_bls12_381(a: &[u64; 36], b2: &[u64; 12]) -> [u64; 36] {
    let a1 = &a[0..12].try_into().unwrap();
    let a2 = &a[12..24].try_into().unwrap();
    let a3 = &a[24..36].try_into().unwrap();

    // c1 = a3·b2·(1+u)
    let mut c1 = mul_fp2_bls12_381(a3, b2);
    c1 = mul_fp2_bls12_381(&c1, &EXT_U);

    // c2 = a1·b2
    let c2 = mul_fp2_bls12_381(a1, b2);

    // c3 = a2·b2
    let c3 = mul_fp2_bls12_381(a2, b2);

    [c1, c2, c3].concat().try_into().unwrap()
}

/// Multiplication of a = a1 + a2·v + a3·v² and b = b2·v + b3·v² in Fp6
//
//  in: (a1 + a2·v + a3·v²),(b2·v + b3·v²) ∈ Fp6, where ai,bi ∈ Fp2
//  out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//       - c1 = (a2·b3 + a3·b2)·(1+u)
//       - c2 = a1·b2 + a3·b3·(1+u)
//       - c3 = a1·b3 + a2·b2
#[inline]
pub fn sparse_mulb_fp6_bls12_381(a: &[u64; 36], b: &[u64; 24]) -> [u64; 36] {
    let a1 = &a[0..12].try_into().unwrap();
    let a2 = &a[12..24].try_into().unwrap();
    let a3 = &a[24..36].try_into().unwrap();
    let b2 = &b[0..12].try_into().unwrap();
    let b3 = &b[12..24].try_into().unwrap();

    // c1 = (a2·b3 + a3·b2)·(1+u)
    let mut c1 = mul_fp2_bls12_381(a2, b3);
    c1 = add_fp2_bls12_381(&c1, &mul_fp2_bls12_381(a3, b2));
    c1 = mul_fp2_bls12_381(&c1, &EXT_U);

    // c2 = a1·b2 + a3·b3·(1+u)
    let mut c2 = mul_fp2_bls12_381(a3, b3);
    c2 = mul_fp2_bls12_381(&c2, &EXT_U);
    c2 = add_fp2_bls12_381(&c2, &mul_fp2_bls12_381(a1, b2));

    // c3 = a1·b3 + a2·b2
    let mut c3 = mul_fp2_bls12_381(a1, b3);
    c3 = add_fp2_bls12_381(&c3, &mul_fp2_bls12_381(a2, b2));

    [c1, c2, c3].concat().try_into().unwrap()
}

/// Multiplication of a = a1 + a2·v + a3·v² and b = b1 + b3·v² in Fp6
//
//  in: (a1 + a2·v + a3·v²),(b1 + b3·v²) ∈ Fp6, where ai,bi ∈ Fp2
//  out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//       - c1 = a1·b1 + a2·b3·(1+u)
//       - c2 = a2·b1 + a3·b3·(1+u)
//       - c3 = a1·b3 + a3·b1
#[inline]
pub fn sparse_mulc_fp6_bls12_381(a: &[u64; 36], b: &[u64; 24]) -> [u64; 36] {
    let a1 = &a[0..12].try_into().unwrap();
    let a2 = &a[12..24].try_into().unwrap();
    let a3 = &a[24..36].try_into().unwrap();
    let b1 = &b[0..12].try_into().unwrap();
    let b3 = &b[12..24].try_into().unwrap();

    // c1 = a1·b1 + a2·b3·(1+u)
    let mut c1 = mul_fp2_bls12_381(a2, b3);
    c1 = mul_fp2_bls12_381(&c1, &EXT_U);
    c1 = add_fp2_bls12_381(&c1, &mul_fp2_bls12_381(a1, b1));

    // c2 = a2·b1 + a3·b3·(1+u)
    let mut c2 = mul_fp2_bls12_381(a3, b3);
    c2 = mul_fp2_bls12_381(&c2, &EXT_U);
    c2 = add_fp2_bls12_381(&c2, &mul_fp2_bls12_381(a2, b1));

    // c3 = a1·b3 + a3·b1
    let mut c3 = mul_fp2_bls12_381(a1, b3);
    c3 = add_fp2_bls12_381(&c3, &mul_fp2_bls12_381(a3, b1));

    [c1, c2, c3].concat().try_into().unwrap()
}

/// Squaring in Fp6
//
//  in: (a1 + a2·v + a3·v²) ∈ Fp6, where ai ∈ Fp2
//  out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//       - c1 = a1² + 2·a2·a3·(1+u)
//       - c2 = a3²·(1+u) + 2·a1·a2
//       - c3 = a2² + 2·a1·a3
#[inline]
pub fn square_fp6_bls12_381(a: &[u64; 36]) -> [u64; 36] {
    let a1 = &a[0..12].try_into().unwrap();
    let a2 = &a[12..24].try_into().unwrap();
    let a3 = &a[24..36].try_into().unwrap();

    // c1 = a1² + 2·a2·a3·(1+u)
    let mut c1 = mul_fp2_bls12_381(a2, a3);
    c1 = dbl_fp2_bls12_381(&c1);
    c1 = mul_fp2_bls12_381(&c1, &EXT_U);
    c1 = add_fp2_bls12_381(&c1, &square_fp2_bls12_381(a1));

    // c2 = a3²·(1+u) + 2·a1·a2
    let mut c2 = square_fp2_bls12_381(a3);
    c2 = mul_fp2_bls12_381(&c2, &EXT_U);
    c2 = add_fp2_bls12_381(&c2, &dbl_fp2_bls12_381(&mul_fp2_bls12_381(a1, a2)));

    // c3 = a2² + 2·a1·a3
    let mut c3 = square_fp2_bls12_381(a2);
    c3 = add_fp2_bls12_381(&c3, &dbl_fp2_bls12_381(&mul_fp2_bls12_381(a1, a3)));

    [c1, c2, c3].concat().try_into().unwrap()
}

/// Inversion in Fp6
//
//  in: (a1 + a2·v + a3·v²) ∈ Fp6, where ai ∈ Fp2
//  out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//       - c1 = c1mid·(a1·c1mid + (1 + u)·(a3·c2mid + a2·c3mid))⁻¹
//       - c2 = c2mid·(a1·c1mid + (1 + u)·(a3·c2mid + a2·c3mid))⁻¹
//       - c3 = c3mid·(a1·c1mid + (1 + u)·(a3·c2mid + a2·c3mid))⁻¹
//  with
//       * c1mid = a1² - (1 + u)·(a2·a3)
//       * c2mid = (1 + u)·a3² - (a1·a2)
//       * c3mid = a2² - (a1·a3)
#[inline]
pub fn inv_fp6_bls12_381(a: &[u64; 36]) -> [u64; 36] {
    let a1 = &a[0..12].try_into().unwrap();
    let a2 = &a[12..24].try_into().unwrap();
    let a3 = &a[24..36].try_into().unwrap();

    // a1², a2², a3²
    let a1_squared = square_fp2_bls12_381(a1);
    let a2_squared = square_fp2_bls12_381(a2);
    let a3_squared = square_fp2_bls12_381(a3);

    // a1·a2, a1·a3, a2·a3
    let a1_a2 = mul_fp2_bls12_381(a1, a2);
    let a1_a3 = mul_fp2_bls12_381(a1, a3);
    let a2_a3 = mul_fp2_bls12_381(a2, a3);

    // c1mid = a1² - (1 + u)·(a2·a3)
    let mut c1mid = mul_fp2_bls12_381(&a2_a3, &EXT_U);
    c1mid = sub_fp2_bls12_381(&a1_squared, &c1mid);

    // c2mid = (1 + u)·a3² - (a1·a2)
    let mut c2mid = mul_fp2_bls12_381(&a3_squared, &EXT_U);
    c2mid = sub_fp2_bls12_381(&c2mid, &a1_a2);

    // c3mid = a2² - (a1·a3)
    let c3mid = sub_fp2_bls12_381(&a2_squared, &a1_a3);

    // (a1·c1mid + (1 + u)·(a3·c2mid + a2·c3mid))⁻¹
    let mut last = mul_fp2_bls12_381(a3, &c2mid);
    last = add_fp2_bls12_381(&last, &mul_fp2_bls12_381(a2, &c3mid));
    last = mul_fp2_bls12_381(&last, &EXT_U);
    last = add_fp2_bls12_381(&last, &mul_fp2_bls12_381(a1, &c1mid));
    let last_inv = inv_fp2_bls12_381(&last);

    // c1 = c1mid·last_inv, c2 = c2mid·last_inv, c3 = c3mid·last_inv
    let c1 = mul_fp2_bls12_381(&c1mid, &last_inv);
    let c2 = mul_fp2_bls12_381(&c2mid, &last_inv);
    let c3 = mul_fp2_bls12_381(&c3mid, &last_inv);

    [c1, c2, c3].concat().try_into().unwrap()
}
