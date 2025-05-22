use super::fp2::{add_fp2_bn254, dbl_fp2_bn254, mul_fp2_bn254, sub_fp2_bn254};

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

pub fn dbl_fp6_bn254(a: &[u64; 24]) -> [u64; 24] {
    let mut result = [0; 24];
    for i in 0..3 {
        let a_i = &a[i * 8..(i + 1) * 8].try_into().unwrap();
        let c_i = dbl_fp2_bn254(a_i);
        result[i * 8..(i + 1) * 8].copy_from_slice(&c_i);
    }
    result
}

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

// mulFp6BN254:
//             in: (a1 + a2·v + a3·v²),(b1 + b2·v + b3·v²) ∈ Fp6, where ai,bi ∈ Fp2
//             out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//                  - c1 = [(a2+a3)·(b2+b3) - a2·b2 - a3·b3]·(9+u) + a1·b1
//                  - c2 = (a1+a2)·(b1+b2) - a1·b1 - a2·b2 + a3·b3·(9+u)
//                  - c3 = (a1+a3)·(b1+b3) - a1·b1 + a2·b2 - a3·b3
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

// sparseMulAFp6BN254:
//             in: (a1 + a2·v + a3·v²),b2·v ∈ Fp6, where ai,b2 ∈ Fp2
//             out: (c1 + c2·v + c3·v²) ∈ Fp6, where:
//                  - c1 = b2·a3·(9+u)
//                  - c2 = b2·a1
//                  - c3 = b2·a2
pub fn sparse_mul_fp6_bn254(a: &[u64; 24], b2: &[u64; 8]) -> [u64; 24] {
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
