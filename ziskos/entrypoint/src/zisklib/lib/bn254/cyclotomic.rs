//! Operations on the cyclotomic subgroup GΦ6(p²) of Fp12

use crate::zisklib::lib::utils::eq;

use super::{
    fp12::mul_fp12_bn254,
    fp2::{
        add_fp2_bn254, dbl_fp2_bn254, inv_fp2_bn254, mul_fp2_bn254, scalar_mul_fp2_bn254,
        square_fp2_bn254, sub_fp2_bn254,
    },
};

/// Compression in the cyclotomic subgroup GΦ6(p²)
///
// in: a = (a0 + a4·v + a3·v²) + (a2 + a1·v + a5·v²)·w ∈ GΦ6(p²), where ai ∈ Fp2
// out: C(a) = [a2,a3,a4,a5] ∈ Fp2⁴
//
/// **NOTE**: If the input does not belong to the cyclotomic subgroup GΦ6(p²), then the compression-decompression
///           technique is not well defined. This means that D(C(a)) != a.
pub fn compress_cyclo_bn254(a: &[u64; 48]) -> [u64; 32] {
    // let a0: &[u64; 8] = &a[0..8].try_into().unwrap();
    let a4: &[u64; 8] = &a[8..16].try_into().unwrap();
    let a3: &[u64; 8] = &a[16..24].try_into().unwrap();
    let a2: &[u64; 8] = &a[24..32].try_into().unwrap();
    // let a1: &[u64; 8] = &a[32..40].try_into().unwrap();
    let a5: &[u64; 8] = &a[40..48].try_into().unwrap();

    let mut result = [0; 32];
    result[0..8].copy_from_slice(a2);
    result[8..16].copy_from_slice(a3);
    result[16..24].copy_from_slice(a4);
    result[24..32].copy_from_slice(a5);
    result
}

/// Decompression in the cyclotomic subgroup GΦ6(p²)
///
// in: [a2,a3,a4,a5] ∈ Fp2⁴, where ai ∈ Fp2
// out: D(a) = (a0 + a4·v + a3·v²) + (a2 + a1·v + a5·v²)·w ∈ GΦ6(p²), where:
//      - if a2 != 0, then:
//          · a1 = (a5²·(9+u) + 3·a4² - 2·a3)/(4·a2)
//          · a0 = (2·a1² + a2·a5 - 3·a3·a4)(9+u) + 1
//      - if a2 == 0, then:
//          · a1 = (2·a4·a5)/a3
//          · a0 = (2·a1² - 3·a3·a4)(9+u) + 1
//
/// **NOTE**: If the input is not of the form C(a), where a ∈ GΦ6(p²), then the compression-decompression
///           technique is not well defined. This means that D(C(a)) != a.
#[inline]
pub fn decompress_cyclo_bn254(a: &[u64; 32]) -> [u64; 48] {
    let a2: &[u64; 8] = &a[0..8].try_into().unwrap();
    let a3: &[u64; 8] = &a[8..16].try_into().unwrap();
    let a4: &[u64; 8] = &a[16..24].try_into().unwrap();
    let a5: &[u64; 8] = &a[24..32].try_into().unwrap();

    let (a0, a1) = if eq(a2, &[0, 0, 0, 0, 0, 0, 0, 0]) {
        // a1 = (2·a4·a5)/a3
        let a3_inv = inv_fp2_bn254(a3);
        let mut a1 = mul_fp2_bn254(a4, a5);
        a1 = dbl_fp2_bn254(&a1);
        a1 = mul_fp2_bn254(&a1, &a3_inv);

        // a0 = (2·a1² - 3·a3·a4)(9+u) + 1
        let a3a4 = mul_fp2_bn254(a3, a4);
        let mut a0 = square_fp2_bn254(&a1);
        a0 = dbl_fp2_bn254(&a0);
        a0 = sub_fp2_bn254(&a0, &scalar_mul_fp2_bn254(&a3a4, &[3, 0, 0, 0]));
        a0 = mul_fp2_bn254(&a0, &[9, 0, 0, 0, 1, 0, 0, 0]);
        a0 = add_fp2_bn254(&a0, &[1, 0, 0, 0, 0, 0, 0, 0]);

        (a0, a1)
    } else {
        // a1 = (a5²·(9+u) + 3·a4² - 2·a3)/(4·a2)
        let a2_inv = inv_fp2_bn254(&scalar_mul_fp2_bn254(a2, &[4, 0, 0, 0]));
        let a4_sq = square_fp2_bn254(a4);
        let mut a1 = square_fp2_bn254(a5);
        a1 = mul_fp2_bn254(&a1, &[9, 0, 0, 0, 1, 0, 0, 0]);
        a1 = add_fp2_bn254(&a1, &scalar_mul_fp2_bn254(&a4_sq, &[3, 0, 0, 0]));
        a1 = sub_fp2_bn254(&a1, &dbl_fp2_bn254(a3));
        a1 = mul_fp2_bn254(&a1, &a2_inv);

        // a0 = (2·a1² + a2·a5 - 3·a3·a4)(9+u) + 1
        let a3a4 = mul_fp2_bn254(a3, a4);
        let a2a5 = mul_fp2_bn254(a2, a5);
        let mut a0 = square_fp2_bn254(&a1);
        a0 = dbl_fp2_bn254(&a0);
        a0 = add_fp2_bn254(&a0, &a2a5);
        a0 = sub_fp2_bn254(&a0, &scalar_mul_fp2_bn254(&a3a4, &[3, 0, 0, 0]));
        a0 = mul_fp2_bn254(&a0, &[9, 0, 0, 0, 1, 0, 0, 0]);
        a0 = add_fp2_bn254(&a0, &[1, 0, 0, 0, 0, 0, 0, 0]);

        (a0, a1)
    };

    let mut result = [0; 48];
    result[0..8].copy_from_slice(&a0);
    result[8..16].copy_from_slice(a4);
    result[16..24].copy_from_slice(a3);
    result[24..32].copy_from_slice(a2);
    result[32..40].copy_from_slice(&a1);
    result[40..48].copy_from_slice(a5);

    result
}

/// Squaring in the cyclotomic subgroup GΦ6(p²)
///
// in: [a2,a3,a4,a5] ∈ Fp2⁴, where ai ∈ Fp2
// out: C(a²) = [b2,b3,b4,b5] ∈ Fp2⁴, where:
//      - b2 = 2(a2 + 3·(9+u)·B45)
//      - b3 = 3·(A45 - (10+u)·B45) - 2·a3
//      - b4 = 3·(A23 - (10+u)·B23) - 2·a4
//      - b5 = 2·(a5 + 3·B23)
//     - A23 = (a2 + a3)·(a2 + (9+u)·a3)
//     - A45 = (a4 + a5)·(a4 + (9+u)·a5)
//     - B23 = a2·a3
//     - B45 = a4·a5
//
/// **NOTE**: The output is not guaranteed to be in GΦ6(p²), if the input isn't.
pub fn square_cyclo_bn254(a: &[u64; 32]) -> [u64; 32] {
    let a2: &[u64; 8] = &a[0..8].try_into().unwrap();
    let a3: &[u64; 8] = &a[8..16].try_into().unwrap();
    let a4: &[u64; 8] = &a[16..24].try_into().unwrap();
    let a5: &[u64; 8] = &a[24..32].try_into().unwrap();

    // B23 = a2·a3, B45 = a4·a5
    let b23 = mul_fp2_bn254(a2, a3);
    let b45 = mul_fp2_bn254(a4, a5);

    // A23 = (a2 + a3)·(a2 + (9+u)·a3)
    let a3xi = mul_fp2_bn254(a3, &[9, 0, 0, 0, 1, 0, 0, 0]);
    let a23 = mul_fp2_bn254(&add_fp2_bn254(a2, a3), &add_fp2_bn254(a2, &a3xi));

    // A45 = (a4 + a5)·(a4 + (9+u)·a5)
    let a5xi = mul_fp2_bn254(a5, &[9, 0, 0, 0, 1, 0, 0, 0]);
    let a45 = mul_fp2_bn254(&add_fp2_bn254(a4, a5), &add_fp2_bn254(a4, &a5xi));

    // b2 = 2(a2 + 3·(9+u)·B45)
    let mut b2 = mul_fp2_bn254(&b45, &[9, 0, 0, 0, 1, 0, 0, 0]);
    b2 = scalar_mul_fp2_bn254(&b2, &[3, 0, 0, 0]);
    b2 = add_fp2_bn254(a2, &b2);
    b2 = dbl_fp2_bn254(&b2);

    // b3 = 3·(A45 - (10+u)·B45) - 2·a3
    let mut b3 = mul_fp2_bn254(&b45, &[10, 0, 0, 0, 1, 0, 0, 0]);
    b3 = sub_fp2_bn254(&a45, &b3);
    b3 = scalar_mul_fp2_bn254(&b3, &[3, 0, 0, 0]);
    b3 = sub_fp2_bn254(&b3, &dbl_fp2_bn254(a3));

    // b4 = 3·(A23 - (10+u)·B23) - 2·a4
    let mut b4 = mul_fp2_bn254(&b23, &[10, 0, 0, 0, 1, 0, 0, 0]);
    b4 = sub_fp2_bn254(&a23, &b4);
    b4 = scalar_mul_fp2_bn254(&b4, &[3, 0, 0, 0]);
    b4 = sub_fp2_bn254(&b4, &dbl_fp2_bn254(a4));

    // b5 = 2·(a5 + 3·B23)
    let mut b5 = scalar_mul_fp2_bn254(&b23, &[3, 0, 0, 0]);
    b5 = add_fp2_bn254(a5, &b5);
    b5 = dbl_fp2_bn254(&b5);

    let mut result = [0; 32];
    result[0..8].copy_from_slice(&b2);
    result[8..16].copy_from_slice(&b3);
    result[16..24].copy_from_slice(&b4);
    result[24..32].copy_from_slice(&b5);

    result
}

/// Exponentiation in the cyclotomic subgroup GΦ6(p²) by the exponent x = 4965661367192848881
///
// in: x, a = (a0 + a4·v + a3·v²) + (a2 + a1·v + a5·v²)·w ∈ GΦ6(p²), where x = 4965661367192848881 and ai ∈ Fp2
// out: a^x = (a0 + a4·v + a3·v²) + (a2 + a1·v + a5·v²)·w ∈ ∈ GΦ6(p²)
//
/// **NOTE**: The output is not guaranteed to be in GΦ6(p²), if the input isn't.
pub fn exp_by_x_cyclo_bn254(a: &[u64; 48]) -> [u64; 48] {
    // Binary representation of the exponent x = 4965661367192848881 in big-endian format
    const X_BIN_LE: [u8; 63] = [
        1, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0,
        1, 0, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 1, 0, 0, 1, 0,
        0, 0, 1,
    ];

    // Start the loop with a
    let mut result = *a;

    // Compress the input so we can work in compressed form
    let mut comp = compress_cyclo_bn254(a);
    for &bit in X_BIN_LE.iter().skip(1) {
        // We always square (in compressed form): C(c²)
        comp = square_cyclo_bn254(&comp);

        if bit == 1 {
            // decompress and multiply
            let decomp = decompress_cyclo_bn254(&comp);
            result = mul_fp12_bn254(&result, &decomp);
        }
    }

    result
}
