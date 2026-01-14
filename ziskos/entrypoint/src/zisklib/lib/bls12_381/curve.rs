//! Operations on the BLS12-381 curve E: y² = x³ + 4

use crate::{
    syscalls::{
        syscall_bls12_381_curve_add, syscall_bls12_381_curve_dbl, SyscallBls12_381CurveAddParams,
        SyscallPoint384,
    },
    zisklib::{eq, fcall_msb_pos_384, lt},
};

use super::{
    constants::{E_B, GAMMA, IDENTITY_G1, P},
    fp::{
        add_fp_bls12_381, mul_fp_bls12_381, neg_fp_bls12_381, sqrt_fp_bls12_381,
        square_fp_bls12_381,
    },
};

/// Decompresses a G1 point on the BLS12-381 curve from 48 bytes (compressed format).
///
/// Format: Big-endian x-coordinate with flag bits in the top 3 bits of the first byte:
/// - Bit 7 (0x80): Compression flag (must be 1 for compressed)
/// - Bit 6 (0x40): Infinity flag (1 = point at infinity)
/// - Bit 5 (0x20): Sign flag (1 = y is lexicographically largest)
pub fn decompress_bls12_381(
    input: &[u8; 48],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<([u64; 12], bool), &'static str> {
    let flags = input[0];

    // Check compression bit
    if (flags & 0x80) == 0 {
        return Err("Expected compressed point (0x80 flag not set)");
    }

    // Check infinity bit
    if (flags & 0x40) != 0 {
        // Verify rest is zero
        if (flags & 0x3f) != 0 {
            return Err("Invalid infinity encoding");
        }
        for input in input.iter().skip(1) {
            if *input != 0 {
                return Err("Invalid infinity encoding");
            }
        }
        return Ok((IDENTITY_G1, true));
    }

    // Extract sign bit
    let y_sign = (flags & 0x20) != 0;

    // Extract x-coordinate (big-endian), masking off flag bits
    let mut x = [0u64; 6];
    let mut bytes = [0u8; 48];
    bytes.copy_from_slice(input);
    bytes[0] &= 0x1f; // Clear flag bits

    // Convert from big-endian bytes to little-endian u64 limbs
    for i in 0..6 {
        for j in 0..8 {
            x[5 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // Verify x < p
    if !lt(&x, &P) {
        return Err("x coordinate >= field modulus");
    }

    // Calculate the y-coordinate of the point: y = sqrt(x³ + 4)
    let x_sq = square_fp_bls12_381(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_cb = mul_fp_bls12_381(
        &x_sq,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_sq = add_fp_bls12_381(
        &x_cb,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );

    let (y, has_sqrt) = sqrt_fp_bls12_381(
        &y_sq,
        #[cfg(feature = "hints")]
        hints,
    );
    if !has_sqrt {
        return Err("No square root exists - point not on curve");
    }

    // Determine the sign of y, which is (lexicographically) done by checking if y > -y
    let y_neg = neg_fp_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_is_larger = lt(&y_neg, &y);

    // Select the correct y based on sign bit
    let final_y = if y_is_larger == y_sign { y } else { y_neg };

    // Return the point (x, final_y)
    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&x);
    result[6..12].copy_from_slice(&final_y);
    Ok((result, false))
}

/// Check if a non-zero point `p` is on the BLS12-381 curve
pub fn is_on_curve_bls12_381(
    p: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let x: [u64; 6] = p[0..6].try_into().unwrap();
    let y: [u64; 6] = p[6..12].try_into().unwrap();

    // p in E iff y² == x³ + 4
    let lhs = square_fp_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut rhs = square_fp_bls12_381(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = mul_fp_bls12_381(
        &rhs,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = add_fp_bls12_381(
        &rhs,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&lhs, &rhs)
}

/// Check if a non-zero point `p` is on the BLS12-381 subgroup
pub fn is_on_subgroup_bls12_381(
    p: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // p in subgroup iff:
    //          ((x²-1)/3)(2·σ(P) - P - σ²(P)) == σ²(P)
    // where σ(x,y) = (ɣ·x,y)

    // Compute σ(P), σ²(P)
    let sigma1 = sigma_endomorphism_bls12_381(
        p,
        #[cfg(feature = "hints")]
        hints,
    );
    let rhs = sigma_endomorphism_bls12_381(
        &sigma1,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute lhs = ((x²-1)/3)(2·σ(P) - P - σ²(P))
    let mut lhs = dbl_bls12_381(
        &sigma1,
        #[cfg(feature = "hints")]
        hints,
    );
    lhs = sub_bls12_381(
        &lhs,
        p,
        #[cfg(feature = "hints")]
        hints,
    );
    lhs = sub_bls12_381(
        &lhs,
        &rhs,
        #[cfg(feature = "hints")]
        hints,
    );
    lhs = scalar_mul_by_x2div3_bls12_381(
        &lhs,
        #[cfg(feature = "hints")]
        hints,
    );

    eq(&lhs, &rhs)
}

/// Adds two non-zero points `p1` and `p2` on the BLS12-381 curve
pub fn add_bls12_381(
    p1: &[u64; 12],
    p2: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let x1: [u64; 6] = p1[0..6].try_into().unwrap();
    let y1: [u64; 6] = p1[6..12].try_into().unwrap();
    let x2: [u64; 6] = p2[0..6].try_into().unwrap();
    let y2: [u64; 6] = p2[6..12].try_into().unwrap();

    // Is x1 == x2?
    if eq(&x1, &x2) {
        // Is y1 == y2?
        if eq(&y1, &y2) {
            // Compute the doubling
            return dbl_bls12_381(
                p1,
                #[cfg(feature = "hints")]
                hints,
            );
        } else {
            // Return 𝒪
            return IDENTITY_G1;
        }
    }

    // Compute the addition
    let mut p1 = SyscallPoint384 { x: x1, y: y1 };
    let p2 = SyscallPoint384 { x: x2, y: y2 };
    let mut params = SyscallBls12_381CurveAddParams { p1: &mut p1, p2: &p2 };
    syscall_bls12_381_curve_add(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&p1.x);
    result[6..12].copy_from_slice(&p1.y);
    result
}

/// Negation of a non-zero point `p` on the BLS12-381 curve
pub fn neg_bls12_381(p: &[u64; 12], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 12] {
    let x: [u64; 6] = p[0..6].try_into().unwrap();
    let y: [u64; 6] = p[6..12].try_into().unwrap();

    let y_neg = neg_fp_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&x);
    result[6..12].copy_from_slice(&y_neg);
    result
}

/// Doubling of a non-zero point `p` on the BLS12-381 curve
pub fn dbl_bls12_381(p: &[u64; 12], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 12] {
    let mut p = SyscallPoint384 { x: p[0..6].try_into().unwrap(), y: p[6..12].try_into().unwrap() };
    syscall_bls12_381_curve_dbl(
        &mut p,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&p.x);
    result[6..12].copy_from_slice(&p.y);
    result
}

/// Subtraction of two non-zero points `p1` and `p2` on the BLS12-381 curve
pub fn sub_bls12_381(
    p1: &[u64; 12],
    p2: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let x2: [u64; 6] = p2[0..6].try_into().unwrap();
    let y2: [u64; 6] = p2[6..12].try_into().unwrap();

    // P1 - P2 = P1 + (-P2)
    let y2_neg = neg_fp_bls12_381(
        &y2,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut p2_neg = [0u64; 12];
    p2_neg[0..6].copy_from_slice(&x2);
    p2_neg[6..12].copy_from_slice(&y2_neg);

    add_bls12_381(
        p1,
        &p2_neg,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Multiplies a non-zero point `p` on the BLS12-381 curve by a scalar `k` on the BLS12-381 scalar field
pub fn scalar_mul_bls12_381(
    p: &[u64; 12],
    k: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
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
            return dbl_bls12_381(
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
    let (max_limb, max_bit) = fcall_msb_pos_384(
        k,
        &[0, 0, 0, 0, 0, 0],
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
            syscall_bls12_381_curve_dbl(
                &mut q,
                #[cfg(feature = "hints")]
                hints,
            );

            // Get the next bit b of k.
            // If b == 1, we should add P to Q, otherwise start the next iteration
            if ((k[i] >> j) & 1) == 1 {
                let mut params = SyscallBls12_381CurveAddParams { p1: &mut q, p2: &p };
                syscall_bls12_381_curve_add(
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
    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&q.x);
    result[6..12].copy_from_slice(&q.y);
    result
}

/// Scalar multiplication of a non-zero point `p` by a binary scalar `k`
pub fn scalar_mul_bin_bls12_381(
    p: &[u64; 12],
    k: &[u8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let x1: [u64; 6] = p[0..6].try_into().unwrap();
    let y1: [u64; 6] = p[6..12].try_into().unwrap();
    let p = SyscallPoint384 { x: x1, y: y1 };

    let mut r = SyscallPoint384 { x: x1, y: y1 };
    for &bit in k.iter().skip(1) {
        syscall_bls12_381_curve_dbl(
            &mut r,
            #[cfg(feature = "hints")]
            hints,
        );
        if bit == 1 {
            let mut params = SyscallBls12_381CurveAddParams { p1: &mut r, p2: &p };
            syscall_bls12_381_curve_add(
                &mut params,
                #[cfg(feature = "hints")]
                hints,
            );
        }
    }

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&r.x);
    result[6..12].copy_from_slice(&r.y);
    result
}

/// Scalar multiplication of a non-zero point by (x²-1)/3
pub fn scalar_mul_by_x2div3_bls12_381(
    p: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    /// Family parameter (X²-1)/3
    const X2DIV3_BIN_BE: [u8; 126] = [
        1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1,
        0, 1, 0, 1, 0, 1,
    ];

    scalar_mul_bin_bls12_381(
        p,
        &X2DIV3_BIN_BE,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Compute the sigma endomorphism σ of a non-zero point `p`, defined as:
///              σ : E(Fp)  ->  E(Fp)
///                  (x,y) |-> (ɣ·x,y)
pub fn sigma_endomorphism_bls12_381(
    p: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut x: [u64; 6] = p[0..6].try_into().unwrap();
    let y: [u64; 6] = p[6..12].try_into().unwrap();

    x = mul_fp_bls12_381(
        &x,
        &GAMMA,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&x);
    result[6..12].copy_from_slice(&y);
    result
}

// ========== Pointer-based API ==========

/// # Safety
/// - `ret` must point to a valid `[u64; 12]` (96 bytes) for the output.
/// - `input` must point to a valid `[u8; 48]` (48 bytes) for the compressed input.
///   Returns:
///   - 0 = success (regular point)
///   - 1 = success (point at infinity)
///   - 2 = error
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_decompress_bls12_381_c")]
pub unsafe extern "C" fn decompress_bls12_381_c(
    ret: *mut u64,
    input: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let input_arr: &[u8; 48] = &*(input as *const [u8; 48]);

    match decompress_bls12_381(
        input_arr,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok((result, is_infinity)) => {
            let ret_arr: &mut [u64; 12] = &mut *(ret as *mut [u64; 12]);
            *ret_arr = result;
            if is_infinity {
                1
            } else {
                0
            }
        }
        Err(_) => 2,
    }
}

/// # Safety
/// - `p` must point to a valid `[u64; 12]` (96 bytes) for the input point.
///   Returns true if the point is on the curve, false otherwise.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_is_on_curve_bls12_381_c")]
pub unsafe extern "C" fn is_on_curve_bls12_381_c(
    p: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p_arr: &[u64; 12] = &*(p as *const [u64; 12]);
    is_on_curve_bls12_381(
        p_arr,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// # Safety
/// - `p` must point to a valid `[u64; 12]` (96 bytes) for the input point.
///   Returns true if the point is in the G1 subgroup, false otherwise.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_is_on_subgroup_bls12_381_c")]
pub unsafe extern "C" fn is_on_subgroup_bls12_381_c(
    p: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p_arr: &[u64; 12] = &*(p as *const [u64; 12]);
    is_on_subgroup_bls12_381(
        p_arr,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// # Safety
/// - `p1` must point to a valid `[u64; 12]` (96 bytes), used as both input and output.
/// - `p2` must point to a valid `[u64; 12]` (96 bytes).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_bls12_381_c")]
pub unsafe extern "C" fn add_bls12_381_c(
    p1: *mut u64,
    p2: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p1_arr: &[u64; 12] = &*(p1 as *const [u64; 12]);
    let p2_arr: &[u64; 12] = &*(p2 as *const [u64; 12]);

    let result = add_bls12_381(
        p1_arr,
        p2_arr,
        #[cfg(feature = "hints")]
        hints,
    );
    if result == IDENTITY_G1 {
        return true;
    }

    let ret_arr: &mut [u64; 12] = &mut *(p1 as *mut [u64; 12]);
    *ret_arr = result;
    false
}

/// # Safety
/// - `p` must point to a valid `[u64; 12]` (96 bytes), used as both input and output.
/// - Point must be non-zero.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_dbl_bls12_381_c")]
pub unsafe extern "C" fn dbl_bls12_381_c(
    p: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let mut p_point =
        SyscallPoint384 { x: *(p as *const [u64; 6]), y: *(p.add(6) as *const [u64; 6]) };

    syscall_bls12_381_curve_dbl(
        &mut p_point,
        #[cfg(feature = "hints")]
        hints,
    );

    *(p as *mut [u64; 6]) = p_point.x;
    *(p.add(6) as *mut [u64; 6]) = p_point.y;
}

/// # Safety
/// - `ret` must point to a valid `[u64; 12]` (96 bytes) for the output.
/// - `p` must point to a valid `[u64; 12]` (96 bytes) for the input point.
/// - `k` must point to a valid `[u64; 6]` (48 bytes) for the scalar.
/// - Point must be non-zero.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_scalar_mul_bls12_381_c")]
pub unsafe extern "C" fn scalar_mul_bls12_381_c(
    ret: *mut u64,
    p: *const u64,
    k: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let p_arr: &[u64; 12] = &*(p as *const [u64; 12]);
    let k_arr: &[u64; 6] = &*(k as *const [u64; 6]);

    let result = scalar_mul_bls12_381(
        p_arr,
        k_arr,
        #[cfg(feature = "hints")]
        hints,
    );
    let ret_arr: &mut [u64; 12] = &mut *(ret as *mut [u64; 12]);
    *ret_arr = result;
}
