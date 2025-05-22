use super::fp6::{
    add_fp6_bn254, dbl_fp6_bn254, mul_fp6_bn254, sparse_mul_fp6_bn254, sub_fp6_bn254,
};

// mulFp12BN254:
//             in: (a1 + a2·w),(b1 + b2·w) ∈ Fp12, where ai,bi ∈ Fp6
//             out: (a1 + a2·w)·(b1 + b2·w) = (c1 + c2·w) ∈ Fp12, where:
//                  - c1 = a1·b1 + a2·b2·v
//                  - c2 = (a1+a2)·(b1+b2) - a1·b1 - a2·b2
pub fn mul_fp12_bn254(a: &[u64; 48], b: &[u64; 48]) -> [u64; 48] {
    let a1 = &a[0..24].try_into().unwrap();
    let a2 = &a[24..48].try_into().unwrap();
    let b1 = &b[0..24].try_into().unwrap();
    let b2 = &b[24..48].try_into().unwrap();

    let a1b1 = mul_fp6_bn254(a1, b1);
    let a2b2 = mul_fp6_bn254(a2, b2);

    let a2b2v = sparse_mul_fp6_bn254(&a2b2, &[1, 0, 0, 0, 0, 0, 0, 0]);
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

// squareFp12BN254:
//             in: (a1 + a2·w) ∈ Fp12, where ai ∈ Fp6
//             out: (a1 + a2·w)² = (c1 + c2·w) ∈ Fp12, where:
//                  - c1 = (a1-a2)·(a1-a2·v) + a1·a2 + a1·a2·v
//                  - c2 = 2·a1·a2
pub fn square_fp12_bn254(a: &[u64; 48]) -> [u64; 48] {
    let a1 = &a[0..24].try_into().unwrap();
    let a2 = &a[24..48].try_into().unwrap();

    // a1·a2, a2·v, a1·a2·v
    let a1a2 = mul_fp6_bn254(a1, a2);
    let a2v = sparse_mul_fp6_bn254(a2, &[1, 0, 0, 0, 0, 0, 0, 0]);
    let a1a2v = sparse_mul_fp6_bn254(&a1a2, &[1, 0, 0, 0, 0, 0, 0, 0]);

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
