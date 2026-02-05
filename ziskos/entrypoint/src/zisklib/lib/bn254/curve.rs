//! Operations on the BN254 curve E: y² = x³ + 3

use crate::{
    syscalls::{
        syscall_bn254_curve_add, syscall_bn254_curve_dbl, SyscallBn254CurveAddParams,
        SyscallPoint256,
    },
    zisklib::{eq, fcall_msb_pos_256},
};

use super::{
    constants::{E_B, IDENTITY_G1},
    fp::{add_fp_bn254, inv_fp_bn254, mul_fp_bn254, square_fp_bn254},
};

/// Check if a non-zero point `p` is on the BN254 curve
pub fn is_on_curve_bn254(p: &[u64; 8]) -> bool {
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    // p in E iff y² == x³ + 3
    let lhs = square_fp_bn254(&y);
    let mut rhs = square_fp_bn254(&x);
    rhs = mul_fp_bn254(&rhs, &x);
    rhs = add_fp_bn254(&rhs, &E_B);
    eq(&lhs, &rhs)
}

/// Converts a point `p` on the BN254 curve from Jacobian coordinates to affine coordinates
pub fn to_affine_bn254(p: &[u64; 12]) -> [u64; 8] {
    let z: [u64; 4] = p[8..12].try_into().unwrap();

    if z == [0u64; 4] {
        return IDENTITY_G1;
    } else if z == [1u64, 0, 0, 0] {
        return [p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7]];
    }

    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    let zinv = inv_fp_bn254(&z);
    let zinv_sq = square_fp_bn254(&zinv);

    let x_res = mul_fp_bn254(&x, &zinv_sq);
    let mut y_res = mul_fp_bn254(&y, &zinv_sq);
    y_res = mul_fp_bn254(&y_res, &zinv);

    [x_res[0], x_res[1], x_res[2], x_res[3], y_res[0], y_res[1], y_res[2], y_res[3]]
}

/// Adds two points `p1` and `p2` on the BN254 curve
pub fn add_bn254(p1: &[u64; 8], p2: &[u64; 8]) -> [u64; 8] {
    if *p1 == IDENTITY_G1 {
        return *p2;
    } else if *p2 == IDENTITY_G1 {
        return *p1;
    }

    let x1: [u64; 4] = p1[0..4].try_into().unwrap();
    let y1: [u64; 4] = p1[4..8].try_into().unwrap();
    let x2: [u64; 4] = p2[0..4].try_into().unwrap();
    let y2: [u64; 4] = p2[4..8].try_into().unwrap();

    // Is x1 == x2?
    if eq(&x1, &x2) {
        // Is y1 == y2?
        if eq(&y1, &y2) {
            // Compute the doubling
            return dbl_bn254(p1);
        } else {
            // Return 𝒪
            return IDENTITY_G1;
        }
    }

    // As p1 != p2,-p2, compute the addition

    // Convert the input points to SyscallPoint256
    let mut p1 = SyscallPoint256 { x: x1, y: y1 };
    let p2 = SyscallPoint256 { x: x2, y: y2 };

    // Call the syscall to add the two points
    let mut params = SyscallBn254CurveAddParams { p1: &mut p1, p2: &p2 };
    syscall_bn254_curve_add(&mut params);

    // Convert the result back to a single array
    let x3 = params.p1.x;
    let y3 = params.p1.y;
    [x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]]
}

pub fn dbl_bn254(p: &[u64; 8]) -> [u64; 8] {
    let mut p1 = SyscallPoint256 { x: p[0..4].try_into().unwrap(), y: p[4..8].try_into().unwrap() };
    syscall_bn254_curve_dbl(&mut p1);
    [p1.x[0], p1.x[1], p1.x[2], p1.x[3], p1.y[0], p1.y[1], p1.y[2], p1.y[3]]
}

/// Multiplies a point `p` on the BN254 curve by a scalar `k` on the BN254 scalar field
pub fn mul_bn254(p: &[u64; 8], k: &[u64; 4]) -> [u64; 8] {
    if *p == IDENTITY_G1 {
        return IDENTITY_G1;
    }

    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0] => {
            // Return 𝒪
            return IDENTITY_G1;
        }
        [1, 0, 0, 0] => {
            // Return p
            return *p;
        }
        [2, 0, 0, 0] => {
            // Return 2p
            return dbl_bn254(p);
        }
        _ => {}
    }

    // We can assume k > 2 from now on
    // Hint the length the binary representations of k
    // We will verify the output by recomposing k
    // Moreover, we should check that the first received bit is 1
    let (max_limb, max_bit) = fcall_msb_pos_256(k, &[0, 0, 0, 0]);

    // Perform the loop, based on the binary representation of k

    // We do the first iteration separately
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    // The first received bit should be 1
    assert_eq!((k[max_limb] >> max_bit) & 1, 1);

    // Start at P
    let x1: [u64; 4] = p[0..4].try_into().unwrap();
    let y1: [u64; 4] = p[4..8].try_into().unwrap();
    let mut q = SyscallPoint256 { x: x1, y: y1 };
    let mut k_rec = [0u64; 4];
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
    let p = SyscallPoint256 { x: x1, y: y1 };
    for i in (0..=limb).rev() {
        for j in (0..=bit).rev() {
            // Always double
            syscall_bn254_curve_dbl(&mut q);

            // Get the next bit b of k.
            // If b == 1, we should add P to Q, otherwise start the next iteration
            if ((k[i] >> j) & 1) == 1 {
                let mut params = SyscallBn254CurveAddParams { p1: &mut q, p2: &p };
                syscall_bn254_curve_add(&mut params);

                // Reconstruct k
                k_rec[i] |= 1 << j;
            }
        }
        bit = 63;
    }

    // Check that the reconstructed k is equal to the input k
    assert_eq!(k_rec, *k);

    // Convert the result back to a single array
    let x3 = q.x;
    let y3 = q.y;
    [x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]]
}

/// # Safety
/// `p` must point to a valid `[u64; 8]` (64 bytes, affine G1 point).
#[no_mangle]
pub unsafe extern "C" fn is_on_curve_bn254_c(p_ptr: *const u64) -> bool {
    let p = unsafe { &*(p_ptr as *const [u64; 8]) };
    is_on_curve_bn254(p)
}

/// # Safety
/// - `p` must point to a valid `[u64; 12]` (96 bytes, Jacobian G1 point).
/// - `out` must point to a valid `[u64; 8]` (64 bytes) writable buffer.
#[no_mangle]
pub unsafe extern "C" fn to_affine_bn254_c(p_ptr: *const u64, out_ptr: *mut u64) -> bool {
    let p = unsafe { &*(p_ptr as *const [u64; 12]) };
    let result = to_affine_bn254(p);

    *out_ptr.add(0) = result[0];
    *out_ptr.add(1) = result[1];
    *out_ptr.add(2) = result[2];
    *out_ptr.add(3) = result[3];
    *out_ptr.add(4) = result[4];
    *out_ptr.add(5) = result[5];
    *out_ptr.add(6) = result[6];
    *out_ptr.add(7) = result[7];

    result == IDENTITY_G1
}

/// # Safety
/// - `p1_ptr` must point to a valid `[u64; 8]` (64 bytes, affine G1 point).
/// - `p2_ptr` must point to a valid `[u64; 8]` (64 bytes, affine G1 point).
/// - `out_ptr` must point to a valid `[u64; 8]` (64 bytes) writable buffer.
#[no_mangle]
pub unsafe extern "C" fn add_bn254_c(
    p1_ptr: *const u64,
    p2_ptr: *const u64,
    out_ptr: *mut u64,
) -> bool {
    let p1 = unsafe { &*(p1_ptr as *const [u64; 8]) };
    let p2 = unsafe { &*(p2_ptr as *const [u64; 8]) };
    let result = add_bn254(p1, p2);

    *out_ptr.add(0) = result[0];
    *out_ptr.add(1) = result[1];
    *out_ptr.add(2) = result[2];
    *out_ptr.add(3) = result[3];
    *out_ptr.add(4) = result[4];
    *out_ptr.add(5) = result[5];
    *out_ptr.add(6) = result[6];
    *out_ptr.add(7) = result[7];

    result == IDENTITY_G1
}

/// # Safety
/// - `p_ptr` must point to a valid `[u64; 8]` (64 bytes, affine G1 point).
/// - `k_ptr` must point to a valid `[u64; 4]` (32 bytes, scalar).
/// - `out_ptr` must point to a valid `[u64; 8]` (64 bytes) writable buffer.
#[no_mangle]
pub unsafe extern "C" fn mul_bn254_c(
    p_ptr: *const u64,
    k_ptr: *const u64,
    out_ptr: *mut u64,
) -> bool {
    let p = unsafe { &*(p_ptr as *const [u64; 8]) };
    let k = unsafe { &*(k_ptr as *const [u64; 4]) };
    let result = mul_bn254(p, k);

    *out_ptr.add(0) = result[0];
    *out_ptr.add(1) = result[1];
    *out_ptr.add(2) = result[2];
    *out_ptr.add(3) = result[3];
    *out_ptr.add(4) = result[4];
    *out_ptr.add(5) = result[5];
    *out_ptr.add(6) = result[6];
    *out_ptr.add(7) = result[7];

    result == IDENTITY_G1
}
