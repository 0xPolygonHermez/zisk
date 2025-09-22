use crate::{
    bls12_381_curve_add::{syscall_bls12_381_curve_add, SyscallBls12_381CurveAddParams},
    bls12_381_curve_dbl::syscall_bls12_381_curve_dbl,
    fcall_msb_pos_384,
    point::SyscallPoint384,
    zisklib::lib::utils::eq,
};

use super::{
    constants::{E_B, GAMMA},
    fp::{add_fp_bls12_381, mul_fp_bls12_381, neg_fp_bls12_381, square_fp_bls12_381},
};

/// Family parameter (X²-1)/3
const X2DIV3: [u64; 6] = [0x55555555, 0x396C8C005555E156, 0, 0, 0, 0];

/// Check if a point `p` is on the BLS12-381 curve
pub fn is_on_curve_bls12_381(p: &[u64; 12]) -> bool {
    let x: [u64; 6] = p[0..6].try_into().unwrap();
    let y: [u64; 6] = p[6..12].try_into().unwrap();

    // p in E iff y² == x³ + 4
    let lhs = square_fp_bls12_381(&y);
    let mut rhs = square_fp_bls12_381(&x);
    rhs = mul_fp_bls12_381(&rhs, &x);
    rhs = add_fp_bls12_381(&rhs, &E_B);
    eq(&lhs, &rhs)
}

/// Check if a point `p` is on the BLS12-381 subgroup
pub fn is_on_subgroup_bls12_381(p: &[u64; 12]) -> bool {
    // p in subgroup iff:
    //          ((x²-1)/3)(2·σ(P) - P - σ²(P)) == σ²(P)
    // where σ(x,y) = (ɣ·x,y)

    // Compute σ(P), σ²(P)
    let sigma1 = sigma_endomorphism_bls12_381(p);
    let rhs = sigma_endomorphism_bls12_381(&sigma1);

    // Compute lhs = ((x²-1)/3)(2·σ(P) - P - σ²(P))
    let mut lhs = dbl_bls12_381(&sigma1);
    lhs = sub_bls12_381(&lhs, p);
    lhs = sub_bls12_381(&lhs, &rhs);
    lhs = scalar_mul_by_x2div3_bls12_381(&lhs);

    eq(&lhs, &rhs)
}

/// Adds two non-zero points `p1` and `p2` on the BLS12-381 curve
pub fn add_bls12_381(p1: &[u64; 12], p2: &[u64; 12]) -> [u64; 12] {
    let x1: [u64; 6] = p1[0..6].try_into().unwrap();
    let y1: [u64; 6] = p1[6..12].try_into().unwrap();
    let x2: [u64; 6] = p2[0..6].try_into().unwrap();
    let y2: [u64; 6] = p2[6..12].try_into().unwrap();

    // Is x1 == x2?
    if eq(&x1, &x2) {
        // Is y1 == y2?
        if eq(&y1, &y2) {
            // Compute the doubling
            let mut p1 = SyscallPoint384 { x: x1, y: y1 };
            syscall_bls12_381_curve_dbl(&mut p1);
            return [
                p1.x[0], p1.x[1], p1.x[2], p1.x[3], p1.x[4], p1.x[5], p1.y[0], p1.y[1], p1.y[2],
                p1.y[3], p1.y[4], p1.y[5],
            ];
        } else {
            // Return 𝒪
            return [0u64; 12];
        }
    }

    // Compute the addition
    let mut p1 = SyscallPoint384 { x: x1, y: y1 };
    let p2 = SyscallPoint384 { x: x2, y: y2 };
    let mut params = SyscallBls12_381CurveAddParams { p1: &mut p1, p2: &p2 };
    syscall_bls12_381_curve_add(&mut params);
    [
        p1.x[0], p1.x[1], p1.x[2], p1.x[3], p1.x[4], p1.x[5], p1.y[0], p1.y[1], p1.y[2], p1.y[3],
        p1.y[4], p1.y[5],
    ]
}

/// Doubling of a non-zero point
pub fn dbl_bls12_381(p: &[u64; 12]) -> [u64; 12] {
    let x: [u64; 6] = p[0..6].try_into().unwrap();
    let y: [u64; 6] = p[6..12].try_into().unwrap();

    let mut p = SyscallPoint384 { x, y };
    syscall_bls12_381_curve_dbl(&mut p);
    [p.x[0], p.x[1], p.x[2], p.x[3], p.x[4], p.x[5], p.y[0], p.y[1], p.y[2], p.y[3], p.y[4], p.y[5]]
}

/// Subtraction of two non-zero points
pub fn sub_bls12_381(p1: &[u64; 12], p2: &[u64; 12]) -> [u64; 12] {
    let x2: [u64; 6] = p2[0..6].try_into().unwrap();
    let y2: [u64; 6] = p2[6..12].try_into().unwrap();

    let y2_neg = neg_fp_bls12_381(&y2);

    add_bls12_381(p1, &[x2, y2_neg].concat().try_into().unwrap())
}

/// Multiplies a point `p` on the BLS12-381 curve by a scalar `k` on the BLS12-381 scalar field
pub fn scalar_mul_bls12_381(p: &[u64; 12], k: &[u64; 6]) -> [u64; 12] {
    // Is p = 𝒪?
    if *p == [0u64; 12] {
        // Return 𝒪
        return [0u64; 12];
    }

    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0, 0, 0] => {
            // Return 𝒪
            return [0u64; 12];
        }
        [1, 0, 0, 0, 0, 0] => {
            // Return p
            return *p;
        }
        [2, 0, 0, 0, 0, 0] => {
            // Return 2p
            return dbl_bls12_381(p);
        }
        _ => {}
    }

    // We can assume k > 2 from now on
    // Hint the length the binary representations of k
    // We will verify the output by recomposing k
    // Moreover, we should check that the first received bit is 1
    let (max_limb, max_bit) = fcall_msb_pos_384(k, &[0, 0, 0, 0, 0, 0]);

    // Perform the loop, based on the binary representation of k

    // We do the first iteration separately
    let _max_limb = max_limb as usize;
    let k_bit = (k[_max_limb] >> max_bit) & 1;
    assert_eq!(k_bit, 1); // the first received bit should be 1

    // Start at P
    let x1: [u64; 6] = p[0..6].try_into().unwrap();
    let y1: [u64; 6] = p[6..12].try_into().unwrap();
    let mut q = SyscallPoint384 { x: x1, y: y1 };
    let mut k_rec = [0u64; 6];
    k_rec[_max_limb] |= 1 << max_bit;

    // Perform the rest of the loop
    let p = SyscallPoint384 { x: x1, y: y1 };
    let _max_bit = max_bit as usize;
    for i in (0..=_max_limb).rev() {
        let bit_len = if i == _max_limb { _max_bit - 1 } else { 63 };
        for j in (0..=bit_len).rev() {
            // Always double
            syscall_bls12_381_curve_dbl(&mut q);

            // Get the next bit b of k.
            // If b == 1, we should add P to Q, otherwise start the next iteration
            if ((k[i] >> j) & 1) == 1 {
                let mut params = SyscallBls12_381CurveAddParams { p1: &mut q, p2: &p };
                syscall_bls12_381_curve_add(&mut params);

                // Reconstruct k
                k_rec[i] |= 1 << j;
            }
        }
    }

    // Check that the reconstructed k is equal to the input k
    assert_eq!(k_rec, *k);

    // Convert the result back to a single array
    [q.x, q.y].concat().try_into().unwrap()
}

/// Scalar multiplication of a non-zero point by (x²-1)/3
pub fn scalar_mul_by_x2div3_bls12_381(p: &[u64; 12]) -> [u64; 12] {
    scalar_mul_bls12_381(p, &X2DIV3)
}

/// Compute the sigma endomorphism σ defined as:
///              σ : E(Fp)  ->  E(Fp)
///                  (x,y) |-> (ɣ·x,y)
pub fn sigma_endomorphism_bls12_381(p: &[u64; 12]) -> [u64; 12] {
    let mut x: [u64; 6] = p[0..6].try_into().unwrap();

    x = mul_fp_bls12_381(&x, &GAMMA);

    [x, p[6..12].try_into().unwrap()].concat().try_into().unwrap()
}
