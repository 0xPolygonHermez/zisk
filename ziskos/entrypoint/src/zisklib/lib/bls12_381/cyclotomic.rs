//! Cyclotomic subgroup operations for BLS12-381

use crate::zisklib::lib::utils::eq;

use super::{
    constants::EXT_U,
    fp12::mul_fp12_bls12_381,
    fp2::{
        add_fp2_bls12_381, dbl_fp2_bls12_381, inv_fp2_bls12_381, mul_fp2_bls12_381,
        scalar_mul_fp2_bls12_381, square_fp2_bls12_381, sub_fp2_bls12_381,
    },
};

/// Compression in the cyclotomic subgroup GΦ6(p²)
///
// in: a = (a0 + a4·v + a3·v²) + (a2 + a1·v + a5·v²)·w ∈ GΦ6(p²), where ai ∈ Fp2
// out: C(a) = [a2,a3,a4,a5] ∈ Fp2⁴
//
/// **NOTE**: If the input does not belong to the cyclotomic subgroup GΦ6(p²), then the compression-decompression
///           technique is not well defined. This means that D(C(a)) != a.
pub fn compress_cyclo_bls12_381(a: &[u64; 72]) -> [u64; 48] {
    // let a0: [u64; 12] = a[0..12].try_into().unwrap();
    let a4: [u64; 12] = a[12..24].try_into().unwrap();
    let a3: [u64; 12] = a[24..36].try_into().unwrap();
    let a2: [u64; 12] = a[36..48].try_into().unwrap();
    // let a1: [u64; 12] = a[48..60].try_into().unwrap();
    let a5: [u64; 12] = a[60..72].try_into().unwrap();

    [a2, a3, a4, a5].concat().try_into().unwrap()
}

/// Decompression in the cyclotomic subgroup GΦ6(p²)
///
// in: [a2,a3,a4,a5] ∈ Fp2⁴, where ai ∈ Fp2
// out: D(a) = (a0 + a4·v + a3·v²) + (a2 + a1·v + a5·v²)·w ∈ GΦ6(p²), where:
//      - if a2 != 0, then:
//          · a1 = (a5²·(1+u) + 3·a4² - 2·a3)/(4·a2)
//          · a0 = (2·a1² + a2·a5 - 3·a3·a4)(1+u) + 1
//      - if a2 == 0, then:
//          · a1 = (2·a4·a5)/a3
//          · a0 = (2·a1² - 3·a3·a4)(1+u) + 1
//
/// **NOTE**: If the input is not of the form C(a), where a ∈ GΦ6(p²), then the compression-decompression
///           technique is not well defined. This means that D(C(a)) != a.
#[inline]
pub fn decompress_cyclo_bls12_381(a: &[u64; 48]) -> [u64; 72] {
    let a2: &[u64; 12] = &a[0..12].try_into().unwrap();
    let a3: &[u64; 12] = &a[12..24].try_into().unwrap();
    let a4: &[u64; 12] = &a[24..36].try_into().unwrap();
    let a5: &[u64; 12] = &a[36..48].try_into().unwrap();

    let (a0, a1) = if eq(a2, &[0; 12]) {
        // a1 = (2·a4·a5)/a3
        let a3_inv = inv_fp2_bls12_381(a3);
        let mut a1 = mul_fp2_bls12_381(a4, a5);
        a1 = dbl_fp2_bls12_381(&a1);
        a1 = mul_fp2_bls12_381(&a1, &a3_inv);

        // a0 = (2·a1² - 3·a3·a4)(1+u) + 1
        let a3a4 = mul_fp2_bls12_381(a3, a4);
        let mut a0 = square_fp2_bls12_381(&a1);
        a0 = dbl_fp2_bls12_381(&a0);
        a0 = sub_fp2_bls12_381(&a0, &scalar_mul_fp2_bls12_381(&a3a4, &[3, 0, 0, 0, 0, 0]));
        a0 = mul_fp2_bls12_381(&a0, &EXT_U);
        a0 = add_fp2_bls12_381(&a0, &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        (a0, a1)
    } else {
        // a1 = (a5²·(1+u) + 3·a4² - 2·a3)/(4·a2)
        let a2_inv = inv_fp2_bls12_381(&scalar_mul_fp2_bls12_381(a2, &[4, 0, 0, 0, 0, 0]));
        let mut a4_sq = square_fp2_bls12_381(a4);
        a4_sq = scalar_mul_fp2_bls12_381(&a4_sq, &[3, 0, 0, 0, 0, 0]);
        let mut a1 = square_fp2_bls12_381(a5);
        a1 = mul_fp2_bls12_381(&a1, &EXT_U);
        a1 = add_fp2_bls12_381(&a1, &a4_sq);
        a1 = sub_fp2_bls12_381(&a1, &dbl_fp2_bls12_381(a3));
        a1 = mul_fp2_bls12_381(&a1, &a2_inv);

        // a0 = (2·a1² + a2·a5 - 3·a3·a4)(1+u) + 1
        let a3a4 = mul_fp2_bls12_381(a3, a4);
        let a2a5 = mul_fp2_bls12_381(a2, a5);
        let mut a0 = square_fp2_bls12_381(&a1);
        a0 = dbl_fp2_bls12_381(&a0);
        a0 = add_fp2_bls12_381(&a0, &a2a5);
        a0 = sub_fp2_bls12_381(&a0, &scalar_mul_fp2_bls12_381(&a3a4, &[3, 0, 0, 0, 0, 0]));
        a0 = mul_fp2_bls12_381(&a0, &EXT_U);
        a0 = add_fp2_bls12_381(&a0, &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        (a0, a1)
    };

    [a0, *a4, *a3, *a2, a1, *a5].concat().try_into().unwrap()
}

/// Squaring in the cyclotomic subgroup GΦ6(p²)
///
// in: [a2,a3,a4,a5] ∈ Fp2⁴, where ai ∈ Fp2
// out: C(a²) = [b2,b3,b4,b5] ∈ Fp2⁴, where:
//      - b2 = 2(a2 + 3·(1+u)·B45)
//      - b3 = 3·(A45 - (2+u)·B45) - 2·a3
//      - b4 = 3·(A23 - (2+u)·B23) - 2·a4
//      - b5 = 2·(a5 + 3·B23)
//     - A23 = (a2 + a3)·(a2 + (1+u)·a3)
//     - A45 = (a4 + a5)·(a4 + (1+u)·a5)
//     - B23 = a2·a3
//     - B45 = a4·a5
//
/// **NOTE**: The output is not guaranteed to be in GΦ6(p²), if the input isn't.
pub fn square_cyclo_bls12_381(a: &[u64; 48]) -> [u64; 48] {
    let a2: &[u64; 12] = &a[0..12].try_into().unwrap();
    let a3: &[u64; 12] = &a[12..24].try_into().unwrap();
    let a4: &[u64; 12] = &a[24..36].try_into().unwrap();
    let a5: &[u64; 12] = &a[36..48].try_into().unwrap();

    // B23 = a2·a3, B45 = a4·a5
    let b23 = mul_fp2_bls12_381(a2, a3);
    let b45 = mul_fp2_bls12_381(a4, a5);

    // A23 = (a2 + a3)·(a2 + (1+u)·a3)
    let a3xi = mul_fp2_bls12_381(a3, &EXT_U);
    let a23 = mul_fp2_bls12_381(&add_fp2_bls12_381(a2, a3), &add_fp2_bls12_381(a2, &a3xi));

    // A45 = (a4 + a5)·(a4 + (1+u)·a5)
    let a5xi = mul_fp2_bls12_381(a5, &EXT_U);
    let a45 = mul_fp2_bls12_381(&add_fp2_bls12_381(a4, a5), &add_fp2_bls12_381(a4, &a5xi));

    // b2 = 2(a2 + 3·(1+u)·B45)
    let mut b2 = mul_fp2_bls12_381(&b45, &EXT_U);
    b2 = scalar_mul_fp2_bls12_381(&b2, &[3, 0, 0, 0, 0, 0]);
    b2 = add_fp2_bls12_381(a2, &b2);
    b2 = dbl_fp2_bls12_381(&b2);

    // b3 = 3·(A45 - (2+u)·B45) - 2·a3
    let mut b3 = mul_fp2_bls12_381(&b45, &[2, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
    b3 = sub_fp2_bls12_381(&a45, &b3);
    b3 = scalar_mul_fp2_bls12_381(&b3, &[3, 0, 0, 0, 0, 0]);
    b3 = sub_fp2_bls12_381(&b3, &dbl_fp2_bls12_381(a3));

    // b4 = 3·(A23 - (2+u)·B23) - 2·a4
    let mut b4 = mul_fp2_bls12_381(&b23, &[2, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
    b4 = sub_fp2_bls12_381(&a23, &b4);
    b4 = scalar_mul_fp2_bls12_381(&b4, &[3, 0, 0, 0, 0, 0]);
    b4 = sub_fp2_bls12_381(&b4, &dbl_fp2_bls12_381(a4));

    // b5 = 2·(a5 + 3·B23)
    let mut b5 = scalar_mul_fp2_bls12_381(&b23, &[3, 0, 0, 0, 0, 0]);
    b5 = add_fp2_bls12_381(a5, &b5);
    b5 = dbl_fp2_bls12_381(&b5);

    [b2, b3, b4, b5].concat().try_into().unwrap()
}

/// Exponentiation in the cyclotomic subgroup GΦ6(p²) by the exponent x
///
// in: x, a = (a0 + a4·v + a3·v²) + (a2 + a1·v + a5·v²)·w ∈ GΦ6(p²), where x is "small" and ai ∈ Fp2
// out: a^x = (a0 + a4·v + a3·v²) + (a2 + a1·v + a5·v²)·w ∈ ∈ GΦ6(p²)
//
/// **NOTE**: The output is not guaranteed to be in GΦ6(p²), if the input isn't.
pub fn exp_cyclo_bls12_381(a: &[u64; 72], x: &[u8]) -> [u64; 72] {
    if eq(a, &[0; 72]) {
        return [0; 72];
    }

    // Start the loop at 1
    let mut result = {
        let mut tmp = [0; 72];
        tmp[0] = 1;
        tmp
    };

    // Compress the input so we can work in compressed form
    let mut comp = compress_cyclo_bls12_381(a);
    for &bit in x.iter() {
        if bit == 1 {
            // decompress and multiply
            let decomp = decompress_cyclo_bls12_381(&comp);
            result = mul_fp12_bls12_381(&result, &decomp);
        }

        // We always square (in compressed form): C(c²)
        comp = square_cyclo_bls12_381(&comp);
    }

    result
}

/// Exponentiation in the cyclotomic subgroup GΦ6(p²) by x = 15132376222941642752
pub fn exp_by_x_cyclo_bls12_381(a: &[u64; 72]) -> [u64; 72] {
    /// Family parameter X
    const X_ABS_BIN_LE: [u8; 64] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
        1, 0, 1, 1,
    ];

    exp_cyclo_bls12_381(a, &X_ABS_BIN_LE)
}

/// Exponentiation in the cyclotomic subgroup GΦ6(p²) by x+1 = 15132376222941642753
pub fn exp_by_xone_cyclo_bls12_381(a: &[u64; 72]) -> [u64; 72] {
    /// Family parameter X+1
    const XONE_ABS_BIN_LE: [u8; 64] = [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
        1, 0, 1, 1,
    ];

    exp_cyclo_bls12_381(a, &XONE_ABS_BIN_LE)
}

/// Exponentiation in the cyclotomic subgroup GΦ6(p²) by (x+1)/3 = 5044125407647214251
pub fn exp_by_xdiv3_cyclo_bls12_381(a: &[u64; 72]) -> [u64; 72] {
    /// Family parameter (X+1)/3
    const XDIV3_ABS_BIN_LE: [u8; 63] = [
        1, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0,
        1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0,
        0, 0, 1,
    ];

    exp_cyclo_bls12_381(a, &XDIV3_ABS_BIN_LE)
}
