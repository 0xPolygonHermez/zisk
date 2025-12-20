//! Operations on the BLS12-381 curve E: y² = x³ + 4

use crate::{
    syscalls::{
        syscall_bls12_381_curve_add, syscall_bls12_381_curve_dbl, SyscallBls12_381CurveAddParams,
        SyscallPoint384,
    },
    zisklib::{eq, fcall_msb_pos_384},
};

use super::{
    constants::{E_B, GAMMA, IDENTITY_G1},
    fp::{add_fp_bls12_381, mul_fp_bls12_381, neg_fp_bls12_381, square_fp_bls12_381},
};

/// Check if a non-zero point `p` is on the BLS12-381 curve
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

/// Check if a non-zero point `p` is on the BLS12-381 subgroup
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
            return dbl_bls12_381(p1);
        } else {
            // Return 𝒪
            return IDENTITY_G1;
        }
    }

    // Compute the addition
    let mut p1 = SyscallPoint384 { x: x1, y: y1 };
    let p2 = SyscallPoint384 { x: x2, y: y2 };
    let mut params = SyscallBls12_381CurveAddParams { p1: &mut p1, p2: &p2 };
    syscall_bls12_381_curve_add(&mut params);

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&p1.x);
    result[6..12].copy_from_slice(&p1.y);
    result
}

/// Negation of a non-zero point `p` on the BLS12-381 curve
pub fn neg_bls12_381(p: &[u64; 12]) -> [u64; 12] {
    let x: [u64; 6] = p[0..6].try_into().unwrap();
    let y: [u64; 6] = p[6..12].try_into().unwrap();

    let y_neg = neg_fp_bls12_381(&y);

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&x);
    result[6..12].copy_from_slice(&y_neg);
    result
}

/// Doubling of a non-zero point `p` on the BLS12-381 curve
pub fn dbl_bls12_381(p: &[u64; 12]) -> [u64; 12] {
    let mut p = SyscallPoint384 { x: p[0..6].try_into().unwrap(), y: p[6..12].try_into().unwrap() };
    syscall_bls12_381_curve_dbl(&mut p);

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&p.x);
    result[6..12].copy_from_slice(&p.y);
    result
}

/// Subtraction of two non-zero points `p1` and `p2` on the BLS12-381 curve
pub fn sub_bls12_381(p1: &[u64; 12], p2: &[u64; 12]) -> [u64; 12] {
    let x2: [u64; 6] = p2[0..6].try_into().unwrap();
    let y2: [u64; 6] = p2[6..12].try_into().unwrap();

    // P1 - P2 = P1 + (-P2)
    let y2_neg = neg_fp_bls12_381(&y2);

    let mut p2_neg = [0u64; 12];
    p2_neg[0..6].copy_from_slice(&x2);
    p2_neg[6..12].copy_from_slice(&y2_neg);

    add_bls12_381(p1, &p2_neg)
}

/// Multiplies a non-zero point `p` on the BLS12-381 curve by a scalar `k` on the BLS12-381 scalar field
pub fn scalar_mul_bls12_381(p: &[u64; 12], k: &[u64; 6]) -> [u64; 12] {
    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0, 0, 0] => {
            // Return 𝒪
            return IDENTITY_G1;
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
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    // The first received bit should be 1
    assert_eq!((k[max_limb] >> max_bit) & 1, 1);

    // Start at P
    let x1: [u64; 6] = p[0..6].try_into().unwrap();
    let y1: [u64; 6] = p[6..12].try_into().unwrap();
    let mut q = SyscallPoint384 { x: x1, y: y1 };
    let mut k_rec = [0u64; 6];
    k_rec[max_limb] |= 1 << max_bit;

    // Determine starting limb/bit for the loop
    let mut limb = max_limb;
    let mut bit = if max_bit == 0 {
        // If max_bit is 0 then limb > 0; otherwise k = 1, which is excluded here
        limb -= 1;
        63
    } else {
        max_bit - 1
    };

    // Perform the rest of the loop
    let p = SyscallPoint384 { x: x1, y: y1 };
    for i in (0..=limb).rev() {
        for j in (0..=bit).rev() {
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
        bit = 63;
    }

    // Check that the reconstructed k is equal to the input k
    assert_eq!(k_rec, *k);

    // Convert the result back to a single array
    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&q.x);
    result[6..12].copy_from_slice(&q.y);
    result
}

/// Scalar multiplication of a non-zero point `p` by a binary scalar `k`
pub fn scalar_mul_bin_bls12_381(p: &[u64; 12], k: &[u8]) -> [u64; 12] {
    let x1: [u64; 6] = p[0..6].try_into().unwrap();
    let y1: [u64; 6] = p[6..12].try_into().unwrap();
    let p = SyscallPoint384 { x: x1, y: y1 };

    let mut r = SyscallPoint384 { x: x1, y: y1 };
    for &bit in k.iter().skip(1) {
        syscall_bls12_381_curve_dbl(&mut r);
        if bit == 1 {
            let mut params = SyscallBls12_381CurveAddParams { p1: &mut r, p2: &p };
            syscall_bls12_381_curve_add(&mut params);
        }
    }

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&r.x);
    result[6..12].copy_from_slice(&r.y);
    result
}

/// Scalar multiplication of a non-zero point by (x²-1)/3
pub fn scalar_mul_by_x2div3_bls12_381(p: &[u64; 12]) -> [u64; 12] {
    /// Family parameter (X²-1)/3
    const X2DIV3_BIN_BE: [u8; 126] = [
        1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1,
        0, 1, 0, 1, 0, 1,
    ];

    scalar_mul_bin_bls12_381(p, &X2DIV3_BIN_BE)
}

/// Compute the sigma endomorphism σ of a non-zero point `p`, defined as:
///              σ : E(Fp)  ->  E(Fp)
///                  (x,y) |-> (ɣ·x,y)
pub fn sigma_endomorphism_bls12_381(p: &[u64; 12]) -> [u64; 12] {
    let mut x: [u64; 6] = p[0..6].try_into().unwrap();
    let y: [u64; 6] = p[6..12].try_into().unwrap();

    x = mul_fp_bls12_381(&x, &GAMMA);

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&x);
    result[6..12].copy_from_slice(&y);
    result
}

// ========== Pointer-based API ==========

/// # Safety
/// - `p1` must point to a valid `[u64; 12]` (96 bytes), used as both input and output.
/// - `p2` must point to a valid `[u64; 12]` (96 bytes).
/// - Points must be non-zero and distinct.
#[no_mangle]
pub unsafe extern "C" fn add_bls12_381_c(p1: *mut u64, p2: *const u64) {
    let mut p1_point =
        SyscallPoint384 { x: *(p1 as *const [u64; 6]), y: *(p1.add(6) as *const [u64; 6]) };
    let p2_point =
        SyscallPoint384 { x: *(p2 as *const [u64; 6]), y: *(p2.add(6) as *const [u64; 6]) };

    let mut params = SyscallBls12_381CurveAddParams { p1: &mut p1_point, p2: &p2_point };
    syscall_bls12_381_curve_add(&mut params);

    *(p1 as *mut [u64; 6]) = p1_point.x;
    *(p1.add(6) as *mut [u64; 6]) = p1_point.y;
}

/// # Safety
/// - `p` must point to a valid `[u64; 12]` (96 bytes), used as both input and output.
/// - Point must be non-zero.
#[no_mangle]
pub unsafe extern "C" fn dbl_bls12_381_c(p: *mut u64) {
    let mut p_point =
        SyscallPoint384 { x: *(p as *const [u64; 6]), y: *(p.add(6) as *const [u64; 6]) };

    syscall_bls12_381_curve_dbl(&mut p_point);

    *(p as *mut [u64; 6]) = p_point.x;
    *(p.add(6) as *mut [u64; 6]) = p_point.y;
}
