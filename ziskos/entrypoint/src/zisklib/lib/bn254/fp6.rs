//! Operations in the degree 6 extension Fp6 of the BN254 curve

use super::fp2::{
    add_fp2_bn254, dbl_fp2_bn254, inv_fp2_bn254, mul_fp2_bn254, neg_fp2_bn254, square_fp2_bn254,
    sub_fp2_bn254,
};

/// Addition in the degree 6 extension of the BN254 curve
#[inline]
pub fn add_fp6_bn254(a: &[u64; 24], b: &[u64; 24]) -> [u64; 24] {
    let mut result = [0; 24];
    for i in 0..3 {
        let a_i = &a[i * 8..(i + 1) * 8].try_into().unwrap();
        let b_i = &b[i * 8..(i + 1) * 8].try_into().unwrap();
        let c_i = add_fp2_bn254(a_i, b_i);
        result[i * 8..(i + 1) * 8].copy_from_slice(&c_i);
    }
    result
}

/// Doubling in the degree 6 extension of the BN254 curve
#[inline]
pub fn dbl_fp6_bn254(a: &[u64; 24]) -> [u64; 24] {
    let mut result = [0; 24];
    for i in 0..3 {
        let a_i = &a[i * 8..(i + 1) * 8].try_into().unwrap();
        let c_i = dbl_fp2_bn254(a_i);
        result[i * 8..(i + 1) * 8].copy_from_slice(&c_i);
    }
    result
}

/// Negation in the degree 6 extension of the BN254 curve
#[inline]
pub fn neg_fp6_bn254(a: &[u64; 24]) -> [u64; 24] {
    let mut result = [0; 24];
    for i in 0..3 {
        let a_i = &a[i * 8..(i + 1) * 8].try_into().unwrap();
        let c_i = neg_fp2_bn254(a_i);
        result[i * 8..(i + 1) * 8].copy_from_slice(&c_i);
    }
    result
}

/// Subtraction in the degree 6 extension of the BN254 curve
#[inline]
pub fn sub_fp6_bn254(a: &[u64; 24], b: &[u64; 24]) -> [u64; 24] {
    let mut result = [0; 24];
    for i in 0..3 {
        let a_i = &a[i * 8..(i + 1) * 8].try_into().unwrap();
        let b_i = &b[i * 8..(i + 1) * 8].try_into().unwrap();
        let c_i = sub_fp2_bn254(a_i, b_i);
        result[i * 8..(i + 1) * 8].copy_from_slice(&c_i);
    }
    result
}

/// Multiplication in the degree 6 extension of the BN254 curve
//
// in: (a1 + a2·v + a3·v²),(b1 + b2·v + b3·v²) ∈ Fp6, where ai,bi ∈ Fp2
// out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//      - c1 = [(a2+a3)·(b2+b3) - a2·b2 - a3·b3]·(9+u) + a1·b1
//      - c2 = (a1+a2)·(b1+b2) - a1·b1 - a2·b2 + a3·b3·(9+u)
//      - c3 = (a1+a3)·(b1+b3) - a1·b1 + a2·b2 - a3·b3
#[inline]
pub fn mul_fp6_bn254(a: &[u64; 24], b: &[u64; 24]) -> [u64; 24] {
    let a1 = &a[0..8].try_into().unwrap();
    let a2 = &a[8..16].try_into().unwrap();
    let a3 = &a[16..24].try_into().unwrap();
    let b1 = &b[0..8].try_into().unwrap();
    let b2 = &b[8..16].try_into().unwrap();
    let b3 = &b[16..24].try_into().unwrap();

    // a1·b1, a2·b2, a3·b3, a3·b3·(9+u)
    let a1b1 = mul_fp2_bn254(a1, b1);
    let a2b2 = mul_fp2_bn254(a2, b2);
    let a3b3 = mul_fp2_bn254(a3, b3);
    let a3b3xi = mul_fp2_bn254(&a3b3, &[9, 0, 0, 0, 1, 0, 0, 0]);

    // a2+a3, b2+b3, a1+a2, b1+b2, a1+a3, b1+b3
    let a2_plus_a3 = add_fp2_bn254(a2, a3);
    let b2_plus_b3 = add_fp2_bn254(b2, b3);
    let a1_plus_a2 = add_fp2_bn254(a1, a2);
    let b1_plus_b2 = add_fp2_bn254(b1, b2);
    let a1_plus_a3 = add_fp2_bn254(a1, a3);
    let b1_plus_b3 = add_fp2_bn254(b1, b3);

    // c1 = [(a2+a3)·(b2+b3) - a2·b2 - a3·b3]·(9+u) + a1·b1
    let mut c1 = mul_fp2_bn254(&a2_plus_a3, &b2_plus_b3);
    c1 = sub_fp2_bn254(&c1, &a2b2);
    c1 = sub_fp2_bn254(&c1, &a3b3);
    c1 = mul_fp2_bn254(&c1, &[9, 0, 0, 0, 1, 0, 0, 0]);
    c1 = add_fp2_bn254(&c1, &a1b1);

    // c2 = (a1+a2)·(b1+b2) - a1·b1 - a2·b2 + a3·b3·(9+u)
    let mut c2 = mul_fp2_bn254(&a1_plus_a2, &b1_plus_b2);
    c2 = sub_fp2_bn254(&c2, &a1b1);
    c2 = sub_fp2_bn254(&c2, &a2b2);
    c2 = add_fp2_bn254(&c2, &a3b3xi);

    // c3 = (a1+a3)·(b1+b3) - a1·b1 + a2·b2 - a3·b3
    let mut c3 = mul_fp2_bn254(&a1_plus_a3, &b1_plus_b3);
    c3 = sub_fp2_bn254(&c3, &a1b1);
    c3 = add_fp2_bn254(&c3, &a2b2);
    c3 = sub_fp2_bn254(&c3, &a3b3);

    let mut result = [0; 24];
    result[0..8].copy_from_slice(&c1);
    result[8..16].copy_from_slice(&c2);
    result[16..24].copy_from_slice(&c3);
    result
}

/// Multiplication of a = a1 + a2·v + a3·v² and b = b2·v in the degree 6 extension of the BN254 curve
//
// in: (a1 + a2·v + a3·v²),b2·v ∈ Fp6, where ai,b2 ∈ Fp2
// out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//      - c1 = b2·a3·(9+u)
//      - c2 = b2·a1
//      - c3 = b2·a2
#[inline]
pub fn sparse_mula_fp6_bn254(a: &[u64; 24], b2: &[u64; 8]) -> [u64; 24] {
    let a1 = &a[0..8].try_into().unwrap();
    let a2 = &a[8..16].try_into().unwrap();
    let a3 = &a[16..24].try_into().unwrap();

    // c1 = b2·a3·(9+u)
    let mut c1 = mul_fp2_bn254(b2, a3);
    c1 = mul_fp2_bn254(&c1, &[9, 0, 0, 0, 1, 0, 0, 0]);

    // c2 = b2·a1
    let c2 = mul_fp2_bn254(b2, a1);

    // c3 = b2·a2
    let c3 = mul_fp2_bn254(b2, a2);

    let mut result = [0; 24];
    result[0..8].copy_from_slice(&c1);
    result[8..16].copy_from_slice(&c2);
    result[16..24].copy_from_slice(&c3);
    result
}

/// Multiplication of a = a1 + a2·v + a3·v² and b = b1 + b2·v in the degree 6 extension of the BN254 curve
//
// in: (a1 + a2·v + a3·v²),(b1 + b2·v) ∈ Fp6, where ai,bi ∈ Fp2
// out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//      - c1 = a1·b1 + a3·b2·(9+u)
//      - c2 = a1·b2 + a2·b1
//      - c3 = a2·b2 + a3·b1
#[inline]
pub fn sparse_mulb_fp6_bn254(a: &[u64; 24], b: &[u64; 16]) -> [u64; 24] {
    let a1 = &a[0..8].try_into().unwrap();
    let a2 = &a[8..16].try_into().unwrap();
    let a3 = &a[16..24].try_into().unwrap();
    let b1 = &b[0..8].try_into().unwrap();
    let b2 = &b[8..16].try_into().unwrap();

    // c1 = a1·b1 + a3·b2·(9+u)
    let mut c1 = mul_fp2_bn254(a1, b1);
    c1 = add_fp2_bn254(&c1, &mul_fp2_bn254(&a3, &mul_fp2_bn254(b2, &[9, 0, 0, 0, 1, 0, 0, 0])));

    // c2 = a1·b2 + a2·b1
    let mut c2 = mul_fp2_bn254(a1, b2);
    c2 = add_fp2_bn254(&c2, &mul_fp2_bn254(a2, b1));

    // c3 = a2·b2 + a3·b1
    let mut c3 = mul_fp2_bn254(a2, b2);
    c3 = add_fp2_bn254(&c3, &mul_fp2_bn254(a3, b1));

    let mut result = [0; 24];
    result[0..8].copy_from_slice(&c1);
    result[8..16].copy_from_slice(&c2);
    result[16..24].copy_from_slice(&c3);
    result
}

/// Multiplication of a = a1 + a2·v + a3·v² and b = b2·v + b3·v² in the degree 6 extension of the BN254 curve
//
// in: (a1 + a2·v + a3·v²),(b2·v + b3·v²) ∈ Fp6, where ai,bi ∈ Fp2
// out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//      - c1 = (a2·b3 + a3·b2)·(9+u)
//      - c2 = a1·b2 + a3·b3·(9+u)
//      - c3 = a1·b3 + a2·b2
#[inline]
pub fn sparse_mulc_fp6_bn254(a: &[u64; 24], b: &[u64; 16]) -> [u64; 24] {
    let a1 = &a[0..8].try_into().unwrap();
    let a2 = &a[8..16].try_into().unwrap();
    let a3 = &a[16..24].try_into().unwrap();
    let b2 = &b[0..8].try_into().unwrap();
    let b3 = &b[8..16].try_into().unwrap();

    // c1 = (a2·b3 + a3·b2)·(9+u)
    let mut c1 = mul_fp2_bn254(a2, b3);
    c1 = add_fp2_bn254(&c1, &mul_fp2_bn254(a3, b2));
    c1 = mul_fp2_bn254(&c1, &[9, 0, 0, 0, 1, 0, 0, 0]);

    // c2 = a1·b2 + a3·b3·(9+u)
    let mut c2 = mul_fp2_bn254(a3, b3);
    c2 = mul_fp2_bn254(&c2, &[9, 0, 0, 0, 1, 0, 0, 0]);
    c2 = add_fp2_bn254(&c2, &mul_fp2_bn254(a1, b2));

    // c3 = a2·b3 + a2·b2
    let mut c3 = mul_fp2_bn254(a1, b3);
    c3 = add_fp2_bn254(&c3, &mul_fp2_bn254(a2, b2));

    let mut result = [0; 24];
    result[0..8].copy_from_slice(&c1);
    result[8..16].copy_from_slice(&c2);
    result[16..24].copy_from_slice(&c3);
    result
}

/// Squaring in the degree 6 extension of the BN254 curve
//
// in: (a1 + a2·v + a3·v²) ∈ Fp6, where ai ∈ Fp2
// out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//      - c1 = 2·a2·a3·(9 + u) + a1²
//      - c2 = a3²·(9 + u) + 2·a1·a2
//      - c3 = 2·a1·a2 - a3² + (a1 - a2 + a3)² + 2·a2·a3 - a1²
#[inline]
pub fn square_fp6_bn254(a: &[u64; 24]) -> [u64; 24] {
    let a1 = &a[0..8].try_into().unwrap();
    let a2 = &a[8..16].try_into().unwrap();
    let a3 = &a[16..24].try_into().unwrap();

    let mut two_a1a2 = mul_fp2_bn254(a1, a2);
    two_a1a2 = dbl_fp2_bn254(&two_a1a2);

    let a3_squared = square_fp2_bn254(a3);

    // c2 = a3²·(9 + u) + 2·a1·a2
    let mut c2 = mul_fp2_bn254(&a3_squared, &[9, 0, 0, 0, 1, 0, 0, 0]);
    c2 = add_fp2_bn254(&c2, &two_a1a2);

    // a1², (a1 - a2 + a3)², 2·a2·a3
    let a1_squared = square_fp2_bn254(a1);
    let mut a1a2a3 = sub_fp2_bn254(a1, a2);
    a1a2a3 = add_fp2_bn254(&a1a2a3, a3);
    a1a2a3 = square_fp2_bn254(&a1a2a3);
    let mut two_a2a3 = mul_fp2_bn254(a2, a3);
    two_a2a3 = dbl_fp2_bn254(&two_a2a3);

    // c1 = 2·a2·a3·(9 + u) + a1²
    let mut c1 = mul_fp2_bn254(&two_a2a3, &[9, 0, 0, 0, 1, 0, 0, 0]);
    c1 = add_fp2_bn254(&c1, &a1_squared);

    // c3 = 2·a1·a2 - a3² + (a1 - a2 + a3)² + 2·a2·a3 - a1²
    let mut c3 = sub_fp2_bn254(&two_a1a2, &a3_squared);
    c3 = add_fp2_bn254(&c3, &a1a2a3);
    c3 = add_fp2_bn254(&c3, &two_a2a3);
    c3 = sub_fp2_bn254(&c3, &a1_squared);

    let mut result = [0; 24];
    result[0..8].copy_from_slice(&c1);
    result[8..16].copy_from_slice(&c2);
    result[16..24].copy_from_slice(&c3);
    result
}

/// Inversion in the degree 6 extension of the BN254 curve
//
// in: (a1 + a2·v + a3·v²) ∈ Fp6, where ai ∈ Fp2
// out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//      - c1 = (a1² - (9 + u)·(a2·a3))·(a1·c1mid + xi·(a3·c2mid + a2·c3mid))⁻¹
//      - c2 = ((9 + u)·a3² - (a1·a2))·(a1·c1mid + xi·(a3·c2mid + a2·c3mid))⁻¹
//      - c3 = (a2²-a1·a3)·(a1·c1mid + xi·(a3·c2mid + a2·c3mid))⁻¹
// with
//      * c1mid = a1² - (9 + u)·(a2·a3)
//      * c2mid = (9 + u)·a3² - (a1·a2)
//      * c3mid = a2² - (a1·a3)
#[inline]
pub fn inv_fp6_bn254(a: &[u64; 24]) -> [u64; 24] {
    let a1 = &a[0..8].try_into().unwrap();
    let a2 = &a[8..16].try_into().unwrap();
    let a3 = &a[16..24].try_into().unwrap();

    let a1_squared = square_fp2_bn254(a1);
    let a2_squared = square_fp2_bn254(a2);
    let a3_squared = square_fp2_bn254(a3);

    let a1a2 = mul_fp2_bn254(a1, a2);
    let a1a3 = mul_fp2_bn254(a1, a3);
    let a2a3 = mul_fp2_bn254(a2, a3);

    // c1mid = a1² - (9 + u)·(a2·a3)
    let mut c1mid = mul_fp2_bn254(&a2a3, &[9, 0, 0, 0, 1, 0, 0, 0]);
    c1mid = sub_fp2_bn254(&a1_squared, &c1mid);

    // c2mid = (9 + u)·a3² - (a1·a2)
    let mut c2mid = mul_fp2_bn254(&a3_squared, &[9, 0, 0, 0, 1, 0, 0, 0]);
    c2mid = sub_fp2_bn254(&c2mid, &a1a2);

    // c3mid = a2² - (a1·a3)
    let c3mid = sub_fp2_bn254(&a2_squared, &a1a3);

    // im = a1·c1mid
    let im = mul_fp2_bn254(a1, &c1mid);

    // last = (im + (9 + u)·(a3·c2mid + a2·c3mid))⁻¹
    let mut last = mul_fp2_bn254(a3, &c2mid);
    last = add_fp2_bn254(&last, &mul_fp2_bn254(a2, &c3mid));
    last = mul_fp2_bn254(&last, &[9, 0, 0, 0, 1, 0, 0, 0]);
    last = add_fp2_bn254(&last, &im);
    last = inv_fp2_bn254(&last);

    // c1 = c1mid·last, c2 = c2mid·last, c3 = c3mid·last
    let c1 = mul_fp2_bn254(&c1mid, &last);
    let c2 = mul_fp2_bn254(&c2mid, &last);
    let c3 = mul_fp2_bn254(&c3mid, &last);

    let mut result = [0; 24];
    result[0..8].copy_from_slice(&c1);
    result[8..16].copy_from_slice(&c2);
    result[16..24].copy_from_slice(&c3);
    result
}
