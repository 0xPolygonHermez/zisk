//! Operations on the twist E': y² = x³ + 4·(1+u) of the BLS12-381 curve

use crate::zisklib::{eq, fcall_msb_pos_384, lt};

use super::{
    constants::{
        ETWISTED_B, EXT_U, EXT_U_INV, FROBENIUS_GAMMA13, FROBENIUS_GAMMA14, IDENTITY_G2, P,
        X_ABS_BIN_BE,
    },
    fp2::{
        add_fp2_bls12_381, conjugate_fp2_bls12_381, dbl_fp2_bls12_381, inv_fp2_bls12_381,
        mul_fp2_bls12_381, neg_fp2_bls12_381, scalar_mul_fp2_bls12_381, sqrt_fp2_bls12_381,
        square_fp2_bls12_381, sub_fp2_bls12_381,
    },
};

/// Decompresses a G2 point on the BLS12-381 twist from 96 bytes (compressed format).
///
/// Format: Big-endian x-coordinate (in Fp2) with flag bits in the top 3 bits of the first byte:
/// - Bit 7 (0x80): Compression flag (must be 1 for compressed)
/// - Bit 6 (0x40): Infinity flag (1 = point at infinity)
/// - Bit 5 (0x20): Sign flag (1 = y is lexicographically largest)
pub fn decompress_twist_bls12_381(
    input: &[u8; 96],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<([u64; 24], bool), &'static str> {
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
        for item in input.iter().skip(1) {
            if *item != 0 {
                return Err("Invalid infinity encoding");
            }
        }
        return Ok((IDENTITY_G2, true));
    }

    // Extract sign bit
    let y_sign = (flags & 0x20) != 0;

    // Extract x-coordinate from big-endian bytes
    // Format: first 48 bytes = x_i (imaginary), next 48 bytes = x_r (real)
    let mut x_i = [0u64; 6];
    let mut x_r = [0u64; 6];

    // Parse x_i (first 48 bytes, masking flag bits in first byte)
    let mut bytes_i = [0u8; 48];
    bytes_i.copy_from_slice(&input[0..48]);
    bytes_i[0] &= 0x1f; // Clear flag bits

    for i in 0..6 {
        for j in 0..8 {
            x_i[5 - i] |= (bytes_i[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // Parse x_r (next 48 bytes)
    for i in 0..6 {
        for j in 0..8 {
            x_r[5 - i] |= (input[48 + i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // Verify x_r < p and x_i < p
    if !lt(&x_r, &P) {
        return Err("x_r coordinate >= field modulus");
    }
    if !lt(&x_i, &P) {
        return Err("x_i coordinate >= field modulus");
    }

    // Build x = x_r + x_i * u as [u64; 12]
    let mut x = [0u64; 12];
    x[0..6].copy_from_slice(&x_r);
    x[6..12].copy_from_slice(&x_i);

    // Calculate y² = x³ + 4(1+u)
    let x_sq = square_fp2_bls12_381(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_cb = mul_fp2_bls12_381(
        &x_sq,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_sq = add_fp2_bls12_381(
        &x_cb,
        &ETWISTED_B,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute sqrt
    let (y, has_sqrt) = sqrt_fp2_bls12_381(
        &y_sq,
        #[cfg(feature = "hints")]
        hints,
    );
    if !has_sqrt {
        return Err("No square root exists - point not on curve");
    }

    // Determine sign of y using lexicographic ordering on Fp2
    // y = y_r + y_i * u is "larger" if:
    //   - y_i > -y_i, OR
    //   - y_i == -y_i (i.e., y_i == 0) AND y_r > -y_r
    let y_neg = neg_fp2_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_r: [u64; 6] = y[0..6].try_into().unwrap();
    let y_i: [u64; 6] = y[6..12].try_into().unwrap();
    let y_neg_r: [u64; 6] = y_neg[0..6].try_into().unwrap();
    let y_neg_i: [u64; 6] = y_neg[6..12].try_into().unwrap();

    let y_is_larger = if !eq(&y_i, &y_neg_i) {
        // Compare i components
        lt(&y_neg_i, &y_i)
    } else {
        // i components equal, compare r
        lt(&y_neg_r, &y_r)
    };

    // Select the correct y based on sign bit
    let final_y = if y_is_larger == y_sign { y } else { y_neg };

    // Return the point (x, final_y)
    let mut result = [0u64; 24];
    result[0..12].copy_from_slice(&x);
    result[12..24].copy_from_slice(&final_y);
    Ok((result, false))
}

/// Check if a non-zero point `p` is on the BLS12-381 twist
pub fn is_on_curve_twist_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // q in E' iff y² == x³ + 4·(1+u)
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();
    let x_sq = square_fp2_bls12_381(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_cubed = mul_fp2_bls12_381(
        &x_sq,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_cubed_plus_b = add_fp2_bls12_381(
        &x_cubed,
        &ETWISTED_B,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_sq = square_fp2_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&x_cubed_plus_b, &y_sq)
}

/// Check if a non-zero point `p` is on the BLS12-381 twist subgroup
pub fn is_on_subgroup_twist_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // p in subgroup iff:
    //          x·𝜓³(P) + P == 𝜓²(P)
    // where ψ := 𝜑⁻¹𝜋ₚ𝜑 is the untwist-Frobenius-twist endomorphism

    // Compute ψ²(P), ψ³(P)
    let utf1 = utf_endomorphism_twist_bls12_381(
        p,
        #[cfg(feature = "hints")]
        hints,
    );
    let rhs = utf_endomorphism_twist_bls12_381(
        &utf1,
        #[cfg(feature = "hints")]
        hints,
    );
    let utf3 = utf_endomorphism_twist_bls12_381(
        &rhs,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute [x]ψ³(P) + P (since x is negative, we compute -[|x|]ψ³(P))
    let xutf3: [u64; 24] = scalar_mul_by_abs_x_twist_bls12_381(
        &utf3,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut lhs = neg_twist_bls12_381(
        &xutf3,
        #[cfg(feature = "hints")]
        hints,
    );
    lhs = add_twist_bls12_381(
        &lhs,
        p,
        #[cfg(feature = "hints")]
        hints,
    );

    eq(&lhs, &rhs)
}

/// Addition of two non-zero points
pub fn add_twist_bls12_381(
    p1: &[u64; 24],
    p2: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let x1: [u64; 12] = p1[0..12].try_into().unwrap();
    let y1: [u64; 12] = p1[12..24].try_into().unwrap();
    let x2: [u64; 12] = p2[0..12].try_into().unwrap();
    let y2: [u64; 12] = p2[12..24].try_into().unwrap();

    // Is x1 == x2?
    if eq(&x1, &x2) {
        // Is y1 == y2?
        if eq(&y1, &y2) {
            // Compute the doubling
            return dbl_twist_bls12_381(
                p1,
                #[cfg(feature = "hints")]
                hints,
            );
        } else {
            // Points are the inverse of each other, return the point at infinity
            return IDENTITY_G2;
        }
    }

    // Compute the addition
    let mut den = sub_fp2_bls12_381(
        &x2,
        &x1,
        #[cfg(feature = "hints")]
        hints,
    );
    den = inv_fp2_bls12_381(
        &den,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut lambda = sub_fp2_bls12_381(
        &y2,
        &y1,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = mul_fp2_bls12_381(
        &lambda,
        &den,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut x3 = square_fp2_bls12_381(
        &lambda,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bls12_381(
        &x3,
        &x1,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bls12_381(
        &x3,
        &x2,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut y3 = sub_fp2_bls12_381(
        &x1,
        &x3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = mul_fp2_bls12_381(
        &lambda,
        &y3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = sub_fp2_bls12_381(
        &y3,
        &y1,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 24];
    result[0..12].copy_from_slice(&x3);
    result[12..24].copy_from_slice(&y3);
    result
}

/// Doubling of a non-zero point
pub fn dbl_twist_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();

    // Compute the doubling
    let mut lambda = dbl_fp2_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = inv_fp2_bls12_381(
        &lambda,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = scalar_mul_fp2_bls12_381(
        &lambda,
        &[0x3, 0, 0, 0, 0, 0],
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = mul_fp2_bls12_381(
        &lambda,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = mul_fp2_bls12_381(
        &lambda,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut x3 = square_fp2_bls12_381(
        &lambda,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bls12_381(
        &x3,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bls12_381(
        &x3,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut y3 = sub_fp2_bls12_381(
        &x,
        &x3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = mul_fp2_bls12_381(
        &lambda,
        &y3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = sub_fp2_bls12_381(
        &y3,
        &y,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 24];
    result[0..12].copy_from_slice(&x3);
    result[12..24].copy_from_slice(&y3);
    result
}

/// Negation of a point
pub fn neg_twist_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();

    // Compute the negation
    let y_neg = neg_fp2_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 24];
    result[0..12].copy_from_slice(&x);
    result[12..24].copy_from_slice(&y_neg);
    result
}

/// Multiplies a non-zero point `p` on the BLS12-381 curve by a scalar `k` on the BLS12-381 scalar field
pub fn scalar_mul_twist_bls12_381(
    p: &[u64; 24],
    k: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0, 0, 0] => {
            // Return 𝒪
            return IDENTITY_G2;
        }
        [1, 0, 0, 0, 0, 0] => {
            // Return p
            return *p;
        }
        [2, 0, 0, 0, 0, 0] => {
            // Return 2p
            return dbl_twist_bls12_381(
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
    let mut q = *p;
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
    for i in (0..=limb).rev() {
        for j in (0..=bit).rev() {
            // Always double
            q = dbl_twist_bls12_381(
                &q,
                #[cfg(feature = "hints")]
                hints,
            );

            // Get the next bit b of k.
            // If b == 1, we should add P to Q, otherwise start the next iteration
            if ((k[i] >> j) & 1) == 1 {
                q = add_twist_bls12_381(
                    &q,
                    p,
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
    q
}

/// Scalar multiplication of a non-zero point `p` by a binary scalar `k`
pub fn scalar_mul_bin_twist_bls12_381(
    p: &[u64; 24],
    k: &[u8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let mut r = *p;
    for &bit in k.iter().skip(1) {
        r = dbl_twist_bls12_381(
            &r,
            #[cfg(feature = "hints")]
            hints,
        );
        if bit == 1 {
            r = add_twist_bls12_381(
                &r,
                p,
                #[cfg(feature = "hints")]
                hints,
            );
        }
    }
    r
}

/// Scalar multiplication of a non-zero point by x
pub fn scalar_mul_by_abs_x_twist_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    scalar_mul_bin_twist_bls12_381(
        p,
        &X_ABS_BIN_BE,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Compute the untwist-frobenius-twist (utf) endomorphism ψ := 𝜑⁻¹𝜋ₚ𝜑 of a non-zero point `p`, where:
///     𝜑 : E'(Fp2) -> E(Fp12) defined by 𝜑(x,y) = (x/ω²,y/ω³) is the untwist map
///     𝜋ₚ : E(Fp12) -> E(Fp12) defined by 𝜋ₚ(x,y) = (xᵖ,yᵖ) is the Frobenius map
///     𝜑⁻¹ : E(Fp12) -> E'(Fp2) defined by 𝜑⁻¹(x,y) = (x·ω²,y·ω³) is the twist map
pub fn utf_endomorphism_twist_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let mut x: [u64; 12] = p[0..12].try_into().unwrap();
    let mut y: [u64; 12] = p[12..24].try_into().unwrap();

    // 1] Compute 𝜑(x,y) = (x/ω²,y/ω³) = (x·(%W_INV_X + %W_INV_Y·u)·ω⁴,y·(%W_INV_X + %W_INV_Y·u)·ω³) ∈ E(Fp12)
    x = mul_fp2_bls12_381(
        &x,
        &EXT_U_INV,
        #[cfg(feature = "hints")]
        hints,
    );
    y = mul_fp2_bls12_381(
        &y,
        &EXT_U_INV,
        #[cfg(feature = "hints")]
        hints,
    );

    // 2] Compute 𝜋ₚ(a,b) = (aᵖ,bᵖ), i.e., apply the frobenius operator
    //    Since the previous result has only one non-zero coefficient, we can apply a specialized frobenius directly
    //    (a·ω⁴)ᵖ = a̅·γ14·ω⁴, (b·ω³)ᵖ = b̅·γ13·ω³
    x = conjugate_fp2_bls12_381(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    x = scalar_mul_fp2_bls12_381(
        &x,
        &FROBENIUS_GAMMA14,
        #[cfg(feature = "hints")]
        hints,
    );
    y = conjugate_fp2_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    y = mul_fp2_bls12_381(
        &y,
        &FROBENIUS_GAMMA13,
        #[cfg(feature = "hints")]
        hints,
    );

    // 3] Compute 𝜑⁻¹(a,b) = (a·ω²,b·ω³) ∈ E'(Fp2). In our particular case, we have:
    //         𝜑⁻¹((a̅·γ14·ω⁴)·ω²,(b̅·γ13·ω³)·ω³) = (a̅·γ14·(1+u), b̅·γ13·(1+u))
    x = mul_fp2_bls12_381(
        &x,
        &EXT_U,
        #[cfg(feature = "hints")]
        hints,
    );
    y = mul_fp2_bls12_381(
        &y,
        &EXT_U,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 24];
    result[0..12].copy_from_slice(&x);
    result[12..24].copy_from_slice(&y);
    result
}

/// # Safety
/// - `ret` must point to a valid `[u64; 24]` (192 bytes) for the output.
/// - `input` must point to a valid `[u8; 96]` (96 bytes) for the compressed input.
///   Returns:
///   - 0 = success (regular point)
///   - 1 = success (point at infinity)
///   - 2 = error
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_decompress_twist_bls12_381_c")]
pub unsafe extern "C" fn decompress_twist_bls12_381_c(
    ret: *mut u64,
    input: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let input_arr: &[u8; 96] = &*(input as *const [u8; 96]);

    match decompress_twist_bls12_381(
        input_arr,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok((result, is_infinity)) => {
            let ret_arr: &mut [u64; 24] = &mut *(ret as *mut [u64; 24]);
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
/// - `p` must point to a valid `[u64; 24]` (192 bytes) for the input point.
///   Returns true if the point is on the twist curve, false otherwise.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_is_on_curve_twist_bls12_381_c")]
pub unsafe extern "C" fn is_on_curve_twist_bls12_381_c(
    p: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p_arr: &[u64; 24] = &*(p as *const [u64; 24]);
    is_on_curve_twist_bls12_381(
        p_arr,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// # Safety
/// - `p` must point to a valid `[u64; 24]` (192 bytes) for the input point.
///   Returns true if the point is in the G2 subgroup, false otherwise.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_is_on_subgroup_twist_bls12_381_c")]
pub unsafe extern "C" fn is_on_subgroup_twist_bls12_381_c(
    p: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p_arr: &[u64; 24] = &*(p as *const [u64; 24]);
    is_on_subgroup_twist_bls12_381(
        p_arr,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// # Safety
/// - `p1` must point to a valid `[u64; 24]` (192 bytes), used as both input and output.
/// - `p2` must point to a valid `[u64; 24]` (192 bytes).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_twist_bls12_381_c")]
pub unsafe extern "C" fn add_twist_bls12_381_c(
    p1: *mut u64,
    p2: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p1_arr: &[u64; 24] = &*(p1 as *const [u64; 24]);
    let p2_arr: &[u64; 24] = &*(p2 as *const [u64; 24]);

    let result = add_twist_bls12_381(
        p1_arr,
        p2_arr,
        #[cfg(feature = "hints")]
        hints,
    );
    if result == IDENTITY_G2 {
        return true;
    }

    let ret_arr: &mut [u64; 24] = &mut *(p1 as *mut [u64; 24]);
    *ret_arr = result;
    false
}

/// # Safety
/// - `ret` must point to a valid `[u64; 24]` for the output affine point.
/// - `p` must point to a valid `[u64; 24]` affine point.
/// - `k` must point to a valid `[u64; 6]` scalar.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_scalar_mul_twist_bls12_381_c")]
pub unsafe extern "C" fn scalar_mul_twist_bls12_381_c(
    ret: *mut u64,
    p: *const u64,
    k: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let p_arr: &[u64; 24] = &*(p as *const [u64; 24]);
    let k_arr: &[u64; 6] = &*(k as *const [u64; 6]);

    let result = scalar_mul_twist_bls12_381(
        p_arr,
        k_arr,
        #[cfg(feature = "hints")]
        hints,
    );
    let ret_arr: &mut [u64; 24] = &mut *(ret as *mut [u64; 24]);
    *ret_arr = result;
}
