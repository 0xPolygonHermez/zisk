//! Operations on the BN254 curve E: y² = x³ + 3

use num_traits::ops::bytes;

use crate::{
    syscalls::{
        syscall_bn254_curve_add, syscall_bn254_curve_dbl, SyscallBn254CurveAddParams,
        SyscallPoint256,
    },
    zisklib::{eq, fcall_msb_pos_256, is_zero, lt},
};

use super::{
    constants::{E_B, G1_IDENTITY, P},
    fp::{add_fp_bn254, inv_fp_bn254, mul_fp_bn254, square_fp_bn254},
    fr::{reduce_fr_bn254, scalar_bytes_be_to_u64_le_bn254},
};

/// G1 add result codes
const G1_ADD_SUCCESS: u8 = 0;
const G1_ADD_SUCCESS_INFINITY: u8 = 1;
const G1_ADD_ERR_INVALID: u8 = 2;
const G1_ADD_ERR_NOT_ON_CURVE: u8 = 3;

/// G1 mul result codes
const G1_MUL_SUCCESS: u8 = 0;
const G1_MUL_SUCCESS_INFINITY: u8 = 1;
const G1_MUL_ERR_NOT_IN_FIELD: u8 = 2;
const G1_MUL_ERR_NOT_ON_CURVE: u8 = 3;

/// Check if a non-zero point `p` is on the BN254 curve
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
    eq(&lhs, &rhs)
}

/// Adds two non-zero points `p1` and `p2` on the BN254 curve
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
pub fn add_complete_bn254(
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
            return Err(G1_ADD_ERR_INVALID);
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
            return Err(G1_ADD_ERR_INVALID);
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
        return Err(G1_ADD_ERR_INVALID);
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
        return Err(G1_ADD_ERR_INVALID);
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

/// Doubles a non-zero point `p` on the BN254 curve
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
        &[0, 0, 0, 0],
        #[cfg(feature = "hints")]
        hints,
    );

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
pub fn mul_complete_bn254(
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

/// BN254 G1 point addition with big-endian byte format
///
/// # Safety
/// - `p1` must point to at least 64 bytes
/// - `p2` must point to at least 64 bytes
/// - `result` must point to a writable buffer of at least 64 bytes
///
/// # Returns
/// - 0 if the operation succeeded
/// - 1 if p1 is invalid (not on curve or invalid field element)
/// - 2 if p2 is invalid (not on curve or invalid field element)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_bn254_g1_add_c")]
pub unsafe extern "C" fn bn254_g1_add_c(
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
    let result = match add_complete_bn254(
        &p1_u64,
        &p2_u64,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Encode result
    if result == G1_IDENTITY {
        G1_ADD_SUCCESS_INFINITY
    } else {
        g1_u64_le_to_bytes_be_bn254(&result, ret_bytes);
        G1_ADD_SUCCESS
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
/// - 0 if the operation succeeded
/// - 1 if point is invalid (not on curve or invalid field element)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_bn254_g1_mul_c")]
pub unsafe extern "C" fn bn254_g1_mul_c(
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
    let product = match mul_complete_bn254(
        &point_u64,
        &scalar_u64,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Encode result
    if product == G1_IDENTITY {
        G1_MUL_SUCCESS_INFINITY
    } else {
        g1_u64_le_to_bytes_be_bn254(&product, ret_bytes);
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
