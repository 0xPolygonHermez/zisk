//! Operations on the BN254 curve E: y² = x³ + 3

#[cfg(zisk_guest)]
use crate::alloc_extern::vec::Vec;

use crate::{
    syscalls::{
        syscall_bn254_curve_add, syscall_bn254_curve_dbl, SyscallBn254CurveAddParams,
        SyscallPoint256,
    },
    zisklib::{eq, fcall_msb_pos_256, is_one, is_zero, lt},
};

use super::{
    constants::{E_B, G1_IDENTITY, P},
    fp::{add_fp_bn254, inv_fp_bn254, mul_fp_bn254, neg_fp_bn254, square_fp_bn254},
    fr::reduce_fr_bn254,
};

/// G1 add result codes
#[allow(dead_code)]
pub(crate) const G1_ADD_SUCCESS: u8 = 0;
#[allow(dead_code)]
pub(crate) const G1_ADD_SUCCESS_INFINITY: u8 = 1;
const G1_ADD_ERR_NOT_IN_FIELD: u8 = 2;
const G1_ADD_ERR_NOT_ON_CURVE: u8 = 3;

/// G1 mul result codes
#[allow(dead_code)]
pub(crate) const G1_MUL_SUCCESS: u8 = 0;
#[allow(dead_code)]
pub(crate) const G1_MUL_SUCCESS_INFINITY: u8 = 1;
const G1_MUL_ERR_NOT_IN_FIELD: u8 = 2;
const G1_MUL_ERR_NOT_ON_CURVE: u8 = 3;

/// Converts a point `p` on the BN254 curve from Jacobian coordinates to affine coordinates
pub fn jacobian_to_affine_bn254(
    p: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let z: [u64; 4] = p[8..12].try_into().unwrap();

    if is_zero(&z) {
        return G1_IDENTITY;
    } else if is_one(&z) {
        return [p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7]];
    }

    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    let zinv = inv_fp_bn254(
        &z,
        #[cfg(feature = "hints")]
        hints,
    );
    let zinv_sq = square_fp_bn254(
        &zinv,
        #[cfg(feature = "hints")]
        hints,
    );

    let x_res = mul_fp_bn254(
        &x,
        &zinv_sq,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut y_res = mul_fp_bn254(
        &y,
        &zinv_sq,
        #[cfg(feature = "hints")]
        hints,
    );
    y_res = mul_fp_bn254(
        &y_res,
        &zinv,
        #[cfg(feature = "hints")]
        hints,
    );

    [x_res[0], x_res[1], x_res[2], x_res[3], y_res[0], y_res[1], y_res[2], y_res[3]]
}

/// Check if a point `p` is on the BN254 curve
pub fn is_on_curve_bn254(p: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> bool {
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    // p in E iff y² == x³ + 3
    let lhs = square_fp_bn254(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut rhs = square_fp_bn254(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = mul_fp_bn254(
        &rhs,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = add_fp_bn254(
        &rhs,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&lhs, &rhs) || eq(p, &G1_IDENTITY)
}

/// Adds two non-zero points `p1` and `p2` on the BN254 curve
///
/// # Soundness
/// Both points must be on-curve, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
pub fn add_bn254(
    p1: &[u64; 8],
    p2: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let x1: [u64; 4] = p1[0..4].try_into().unwrap();
    let y1: [u64; 4] = p1[4..8].try_into().unwrap();
    let x2: [u64; 4] = p2[0..4].try_into().unwrap();
    let y2: [u64; 4] = p2[4..8].try_into().unwrap();

    // Is x1 == x2?
    if eq(&x1, &x2) {
        // Is y1 == y2?
        if eq(&y1, &y2) {
            // Compute the doubling
            return dbl_bn254(
                p1,
                #[cfg(feature = "hints")]
                hints,
            );
        } else {
            // Return 𝒪
            return G1_IDENTITY;
        }
    }

    // As p1 != p2,-p2, compute the addition

    // Convert the input points to SyscallPoint256
    let mut p1 = SyscallPoint256 { x: x1, y: y1 };
    let p2 = SyscallPoint256 { x: x2, y: y2 };

    // Call the syscall to add the two points
    let mut params = SyscallBn254CurveAddParams { p1: &mut p1, p2: &p2 };
    syscall_bn254_curve_add(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    // Convert the result back to a single array
    let x3 = params.p1.x;
    let y3 = params.p1.y;
    [x3[0], x3[1], x3[2], x3[3], y3[0], y3[1], y3[2], y3[3]]
}

/// Addition of two points with validation and identity handling
pub fn add_complete_safe_bn254(
    p1: &[u64; 8],
    p2: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 8], u8> {
    let p1_is_inf = eq(p1, &G1_IDENTITY);
    let p2_is_inf = eq(p2, &G1_IDENTITY);

    // Handle identity cases
    if p1_is_inf && p2_is_inf {
        return Ok(G1_IDENTITY);
    }

    if p1_is_inf {
        // Validate p2 field elements and curve membership
        let x2: [u64; 4] = p2[0..4].try_into().unwrap();
        let y2: [u64; 4] = p2[4..8].try_into().unwrap();
        if !lt(&x2, &P) || !lt(&y2, &P) {
            return Err(G1_ADD_ERR_NOT_IN_FIELD);
        }
        if !is_on_curve_bn254(
            p2,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(G1_ADD_ERR_NOT_ON_CURVE);
        }
        return Ok(*p2);
    }

    if p2_is_inf {
        // Validate p1 field elements and curve membership
        let x1: [u64; 4] = p1[0..4].try_into().unwrap();
        let y1: [u64; 4] = p1[4..8].try_into().unwrap();
        if !lt(&x1, &P) || !lt(&y1, &P) {
            return Err(G1_ADD_ERR_NOT_IN_FIELD);
        }
        if !is_on_curve_bn254(
            p1,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(G1_ADD_ERR_NOT_ON_CURVE);
        }
        return Ok(*p1);
    }

    // Both points are non-identity, validate both
    let x1: [u64; 4] = p1[0..4].try_into().unwrap();
    let y1: [u64; 4] = p1[4..8].try_into().unwrap();
    if !lt(&x1, &P) || !lt(&y1, &P) {
        return Err(G1_ADD_ERR_NOT_IN_FIELD);
    }
    if !is_on_curve_bn254(
        p1,
        #[cfg(feature = "hints")]
        hints,
    ) {
        return Err(G1_ADD_ERR_NOT_ON_CURVE);
    }

    let x2: [u64; 4] = p2[0..4].try_into().unwrap();
    let y2: [u64; 4] = p2[4..8].try_into().unwrap();
    if !lt(&x2, &P) || !lt(&y2, &P) {
        return Err(G1_ADD_ERR_NOT_IN_FIELD);
    }
    if !is_on_curve_bn254(
        p2,
        #[cfg(feature = "hints")]
        hints,
    ) {
        return Err(G1_ADD_ERR_NOT_ON_CURVE);
    }

    // Perform addition
    Ok(add_bn254(
        p1,
        p2,
        #[cfg(feature = "hints")]
        hints,
    ))
}

/// Negation of a point
pub fn neg_bn254(p: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 8] {
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    // Compute the negation
    let y_neg = neg_fp_bn254(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    [x[0], x[1], x[2], x[3], y_neg[0], y_neg[1], y_neg[2], y_neg[3]]
}

/// Doubles a non-zero point `p` on the BN254 curve
///
/// # Soundness
/// The point must be on-curve, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
pub fn dbl_bn254(p: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 8] {
    let mut p1 = SyscallPoint256 { x: p[0..4].try_into().unwrap(), y: p[4..8].try_into().unwrap() };
    syscall_bn254_curve_dbl(
        &mut p1,
        #[cfg(feature = "hints")]
        hints,
    );
    [p1.x[0], p1.x[1], p1.x[2], p1.x[3], p1.y[0], p1.y[1], p1.y[2], p1.y[3]]
}

/// Multiplies a non-zero point `p` on the BN254 curve by a scalar `k` on the BN254 scalar field
///
/// # Soundness
/// The point must be on-curve, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
pub fn scalar_mul_bn254(
    p: &[u64; 8],
    k: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0] => {
            // Return 𝒪
            return G1_IDENTITY;
        }
        [1, 0, 0, 0] => {
            // Return p
            return *p;
        }
        [2, 0, 0, 0] => {
            // Return 2p
            return dbl_bn254(
                p,
                #[cfg(feature = "hints")]
                hints,
            );
        }
        _ => {}
    }

    // We can assume k > 2 from now on
    // Hint the length the binary representations of k
    // We will verify the output by recomposing k
    // Moreover, we should check that the first received bit is 1
    let (max_limb, max_bit) = fcall_msb_pos_256(
        k,
        #[cfg(feature = "hints")]
        hints,
    );

    // Bound before use as index/shift
    assert!(max_limb < 4 && max_bit < 64, "msb_pos hint out of range");

    // Perform the loop, based on the binary representation of k

    // We do the first iteration separately
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    // The first received bit should be 1
    assert_eq!((k[max_limb] >> max_bit) & 1, 1, "The most significant bit of the scalar must be 1");

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
            syscall_bn254_curve_dbl(
                &mut q,
                #[cfg(feature = "hints")]
                hints,
            );

            // Get the next bit b of k.
            // If b == 1, we should add P to Q, otherwise start the next iteration
            if ((k[i] >> j) & 1) == 1 {
                let mut params = SyscallBn254CurveAddParams { p1: &mut q, p2: &p };
                syscall_bn254_curve_add(
                    &mut params,
                    #[cfg(feature = "hints")]
                    hints,
                );

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

/// Scalar multiplication with validation and identity handling
pub fn scalar_mul_complete_safe_bn254(
    p: &[u64; 8],
    k: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 8], u8> {
    // If point is infinity, result is infinity
    if eq(p, &G1_IDENTITY) {
        return Ok(G1_IDENTITY);
    }

    // Point is not infinity, validate field elements and curve membership
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    if !lt(&x, &P) || !lt(&y, &P) {
        return Err(G1_MUL_ERR_NOT_IN_FIELD);
    }

    if !is_on_curve_bn254(
        p,
        #[cfg(feature = "hints")]
        hints,
    ) {
        return Err(G1_MUL_ERR_NOT_ON_CURVE);
    }

    // Reduce the scalar
    let k = reduce_fr_bn254(
        k,
        #[cfg(feature = "hints")]
        hints,
    );

    // Perform scalar multiplication
    Ok(scalar_mul_bn254(
        p,
        &k,
        #[cfg(feature = "hints")]
        hints,
    ))
}

// ==================== C FFI Functions ====================

/// Jacobian to affine conversion for a BN254 G1 point.
///
/// # Safety
/// - `p_ptr` must point to a valid `[u64; 12]` array (Jacobian coordinates x ‖ y ‖ z, little-endian limbs)
/// - `result_ptr` must point to a writable `[u64; 8]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_jacobian_to_affine_bn254_c")]
pub unsafe extern "C" fn jacobian_to_affine_bn254_c(
    p_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let p = &*(p_ptr as *const [u64; 12]);
    let result = &mut *(result_ptr as *mut [u64; 8]);
    match jacobian_to_affine_bn254(
        p,
        #[cfg(feature = "hints")]
        hints,
    ) {
        G1_IDENTITY => {
            *result = G1_IDENTITY;
            1
        }
        affine => {
            *result = affine;
            0
        }
    }
}

/// Curve membership check for a BN254 G1 point.
/// Returns 1 if the point is on the curve, 0 otherwise.
///
/// # Safety
/// - `p_ptr` must point to a valid `[u64; 8]` array (affine coordinates x ‖ y, little-endian limbs)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_is_on_curve_bn254_c")]
pub unsafe extern "C" fn is_on_curve_bn254_c(
    p_ptr: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let p = &*(p_ptr as *const [u64; 8]);
    is_on_curve_bn254(
        p,
        #[cfg(feature = "hints")]
        hints,
    ) as u8
}

/// Addition of two non-zero BN254 G1 points.
///
/// # Safety
/// - `p1_ptr` must point to a valid `[u64; 8]` array (affine coordinates x ‖ y, little-endian limbs)
/// - `p2_ptr` must point to a valid `[u64; 8]` array (affine coordinates x ‖ y, little-endian limbs)
/// - `result_ptr` must point to a writable `[u64; 8]` array
///
/// # Soundness
/// Both points must be on-curve, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_bn254_c")]
pub unsafe extern "C" fn add_bn254_c(
    p1_ptr: *const u64,
    p2_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let p1 = &*(p1_ptr as *const [u64; 8]);
    let p2 = &*(p2_ptr as *const [u64; 8]);
    let result = &mut *(result_ptr as *mut [u64; 8]);
    match add_bn254(
        p1,
        p2,
        #[cfg(feature = "hints")]
        hints,
    ) {
        G1_IDENTITY => {
            *result = G1_IDENTITY;
            1
        }
        sum => {
            *result = sum;
            0
        }
    }
}

/// BN254 G1 point addition with big-endian byte format
///
/// # Safety
/// - `p1` must point to at least 64 bytes
/// - `p2` must point to at least 64 bytes
/// - `result` must point to a writable buffer of at least 64 bytes
///
/// # Returns
/// - [G1_ADD_SUCCESS] = success (result is valid and not infinity)
/// - [G1_ADD_SUCCESS_INFINITY] = success (result is infinity)
/// - [G1_ADD_ERR_NOT_IN_FIELD] = error (one of the input points has coordinates not in the field)
/// - [G1_ADD_ERR_NOT_ON_CURVE] = error (one of the input points is not on the curve)
#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn add_safe_bn254_c(
    p1: *const u8,
    p2: *const u8,
    ret: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let p1_bytes: &[u8; 64] = &*(p1 as *const [u8; 64]);
    let p2_bytes: &[u8; 64] = &*(p2 as *const [u8; 64]);
    let ret_bytes: &mut [u8; 64] = &mut *(ret as *mut [u8; 64]);

    // Convert to internal format
    let p1_u64 = g1_bytes_be_to_u64_le_bn254(p1_bytes);
    let p2_u64 = g1_bytes_be_to_u64_le_bn254(p2_bytes);

    // Perform addition with validation
    let result = match add_complete_safe_bn254(
        &p1_u64,
        &p2_u64,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Encode result
    g1_u64_le_to_bytes_be_bn254(&result, ret_bytes);
    if result == G1_IDENTITY {
        G1_ADD_SUCCESS_INFINITY
    } else {
        G1_ADD_SUCCESS
    }
}

/// Scalar multiplication of a non-zero BN254 G1 point by a scalar.
///
/// # Safety
/// - `p_ptr` must point to a valid `[u64; 8]` array (affine coordinates x ‖ y, little-endian limbs)
/// - `k_ptr` must point to a valid `[u64; 4]` array (scalar, little-endian limbs)
/// - `result_ptr` must point to a writable `[u64; 8]` array
///
/// # Soundness
/// The point must be on-curve, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_scalar_mul_bn254_c")]
pub unsafe extern "C" fn scalar_mul_bn254_c(
    p_ptr: *const u64,
    k_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let p = &*(p_ptr as *const [u64; 8]);
    let k = &*(k_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 8]);
    match scalar_mul_bn254(
        p,
        k,
        #[cfg(feature = "hints")]
        hints,
    ) {
        G1_IDENTITY => {
            *result = G1_IDENTITY;
            1
        }
        product => {
            *result = product;
            0
        }
    }
}

/// BN254 G1 scalar multiplication with big-endian byte format
///
/// # Safety
/// - `point` must point to at least 64 bytes
/// - `scalar` must point to at least 32 bytes
/// - `result` must point to a writable buffer of at least 64 bytes
///
/// # Returns
/// - [G1_MUL_SUCCESS] = success (result is valid and not infinity)
/// - [G1_MUL_SUCCESS_INFINITY] = success (result is infinity)
/// - [G1_MUL_ERR_NOT_IN_FIELD] = error (point or scalar has coordinates not in the field)
/// - [G1_MUL_ERR_NOT_ON_CURVE] = error (point is not on the curve)
#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn scalar_mul_safe_bn254_c(
    point: *const u8,
    scalar: *const u8,
    ret: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let point_bytes: &[u8; 64] = &*(point as *const [u8; 64]);
    let scalar_bytes: &[u8; 32] = &*(scalar as *const [u8; 32]);
    let ret_bytes: &mut [u8; 64] = &mut *(ret as *mut [u8; 64]);

    // Convert to internal format
    let point_u64 = g1_bytes_be_to_u64_le_bn254(point_bytes);
    let scalar_u64 = scalar_bytes_be_to_u64_le_bn254(scalar_bytes);

    // Perform scalar multiplication with validation
    let product = match scalar_mul_complete_safe_bn254(
        &point_u64,
        &scalar_u64,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Encode result
    g1_u64_le_to_bytes_be_bn254(&product, ret_bytes);
    if product == G1_IDENTITY {
        G1_MUL_SUCCESS_INFINITY
    } else {
        G1_MUL_SUCCESS
    }
}

/// Convert 64-byte big-endian G1 point to [u64; 8] little-endian
pub fn g1_bytes_be_to_u64_le_bn254(bytes: &[u8; 64]) -> [u64; 8] {
    let mut result = [0u64; 8];

    // x-coordinate (first 32 bytes)
    for i in 0..4 {
        for j in 0..8 {
            result[3 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // y-coordinate (next 32 bytes)
    for i in 0..4 {
        for j in 0..8 {
            result[7 - i] |= (bytes[32 + i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    result
}

/// Convert little-endian u64 limbs to big-endian bytes for a G1 point ([u64; 8] -> 64 bytes)
#[allow(dead_code)]
fn g1_u64_le_to_bytes_be_bn254(limbs: &[u64; 8], bytes: &mut [u8; 64]) {
    // Encode x coordinate (first 32 bytes, big-endian)
    for i in 0..4 {
        let limb = limbs[3 - i];
        for j in 0..8 {
            bytes[i * 8 + j] = ((limb >> (8 * (7 - j))) & 0xff) as u8;
        }
    }

    // Encode y coordinate (next 32 bytes, big-endian)
    for i in 0..4 {
        let limb = limbs[7 - i];
        for j in 0..8 {
            bytes[32 + i * 8 + j] = ((limb >> (8 * (7 - j))) & 0xff) as u8;
        }
    }
}

/// Convert big-endian bytes to little-endian u64 limbs for a scalar (32 bytes -> [u64; 4])
pub fn scalar_bytes_be_to_u64_le_bn254(bytes: &[u8; 32]) -> [u64; 4] {
    let mut result = [0u64; 4];

    for i in 0..4 {
        for j in 0..8 {
            result[3 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    result
}
