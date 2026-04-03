//! Operations on the twist E': y² = x³ + 4·(1+u) of the BLS12-381 curve

use crate::zisklib::{eq, fcall_msb_pos_256, is_zero, lt};

use super::{
    constants::{
        ETWISTED_B, EXT_U, EXT_U_INV, FROBENIUS_GAMMA13, FROBENIUS_GAMMA14, G2_IDENTITY, P,
        PSI2_C1, PSI_C1, PSI_C2, X_ABS_BIN_BE,
    },
    fp2::{
        add_fp2_bls12_381, conjugate_fp2_bls12_381, dbl_fp2_bls12_381, inv_fp2_bls12_381,
        mul_fp2_bls12_381, neg_fp2_bls12_381, scalar_mul_fp2_bls12_381, sqrt_fp2_bls12_381,
        square_fp2_bls12_381, sub_fp2_bls12_381,
    },
    fr::{reduce_fr_bls12_381, scalar_bytes_be_to_u64_le_bls12_381},
};

/// G2 add result codes
pub const G2_ADD_SUCCESS: u8 = 0;
pub const G2_ADD_SUCCESS_INFINITY: u8 = 1;
pub const G2_ADD_ERR_NOT_IN_FIELD: u8 = 2;
pub const G2_ADD_ERR_NOT_ON_CURVE: u8 = 3;

/// G2 MSM result codes
pub const G2_MSM_SUCCESS: u8 = 0;
pub const G2_MSM_SUCCESS_INFINITY: u8 = 1;
pub const G2_MSM_ERR_NOT_IN_FIELD: u8 = 2;
pub const G2_MSM_ERR_NOT_ON_CURVE: u8 = 3;
pub const G2_MSM_ERR_NOT_IN_SUBGROUP: u8 = 4;

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
        return Err("decompress_twist_bls12_381: Expected compressed point (0x80 flag not set)");
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
        return Ok((G2_IDENTITY, true));
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

fn psi_twist_bls12_381(p: &[u64; 24], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 24] {
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();

    let mut frobx = conjugate_fp2_bls12_381(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    frobx = mul_fp2_bls12_381(
        &frobx,
        &PSI_C1,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut froby = conjugate_fp2_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    froby = mul_fp2_bls12_381(
        &froby,
        &PSI_C2,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 24];
    result[0..12].copy_from_slice(&frobx);
    result[12..24].copy_from_slice(&froby);
    result
}

fn psi2_twist_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();

    let xa = mul_fp2_bls12_381(
        &x,
        &PSI2_C1,
        #[cfg(feature = "hints")]
        hints,
    );
    let ya = neg_fp2_bls12_381(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 24];
    result[0..12].copy_from_slice(&xa);
    result[12..24].copy_from_slice(&ya);
    result
}

/// Efficient cofactor clearing for G2 using endomorphisms
/// Implements: h_eff * P where h_eff is the effective cofactor
pub fn clear_cofactor_twist_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let mut t1 = scalar_mul_by_abs_x_twist_bls12_381(
        p,
        #[cfg(feature = "hints")]
        hints,
    );
    t1 = neg_twist_bls12_381(
        &t1,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut t2 = psi_twist_bls12_381(
        p,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut t3 = dbl_twist_bls12_381(
        p,
        #[cfg(feature = "hints")]
        hints,
    );
    t3 = psi2_twist_bls12_381(
        &t3,
        #[cfg(feature = "hints")]
        hints,
    );
    t3 = sub_twist_bls12_381(
        &t3,
        &t2,
        #[cfg(feature = "hints")]
        hints,
    );
    t2 = add_twist_bls12_381(
        &t1,
        &t2,
        #[cfg(feature = "hints")]
        hints,
    );
    t2 = scalar_mul_by_abs_x_twist_bls12_381(
        &t2,
        #[cfg(feature = "hints")]
        hints,
    );
    t2 = neg_twist_bls12_381(
        &t2,
        #[cfg(feature = "hints")]
        hints,
    );
    t3 = add_twist_bls12_381(
        &t3,
        &t2,
        #[cfg(feature = "hints")]
        hints,
    );
    t3 = sub_twist_bls12_381(
        &t3,
        &t1,
        #[cfg(feature = "hints")]
        hints,
    );
    sub_twist_bls12_381(
        &t3,
        p,
        #[cfg(feature = "hints")]
        hints,
    )
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
            return G2_IDENTITY;
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

/// Addition of two points
pub fn add_complete_twist_bls12_381(
    p1: &[u64; 24],
    p2: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 24], u8> {
    let p1_is_inf = eq(p1, &G2_IDENTITY);
    let p2_is_inf = eq(p2, &G2_IDENTITY);

    // Handle identity cases
    if p1_is_inf && p2_is_inf {
        return Ok(G2_IDENTITY);
    }

    if p1_is_inf {
        // Validate p2 field elements and curve membership
        let x2_0: [u64; 6] = p2[0..6].try_into().unwrap();
        let x2_1: [u64; 6] = p2[6..12].try_into().unwrap();
        let y2_0: [u64; 6] = p2[12..18].try_into().unwrap();
        let y2_1: [u64; 6] = p2[18..24].try_into().unwrap();
        if !lt(&x2_0, &P) || !lt(&x2_1, &P) || !lt(&y2_0, &P) || !lt(&y2_1, &P) {
            return Err(G2_ADD_ERR_NOT_IN_FIELD);
        }
        if !is_on_curve_twist_bls12_381(
            p2,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(G2_ADD_ERR_NOT_ON_CURVE);
        }
        return Ok(*p2);
    }

    if p2_is_inf {
        // Validate p1 field elements and curve membership
        let x1_0: [u64; 6] = p1[0..6].try_into().unwrap();
        let x1_1: [u64; 6] = p1[6..12].try_into().unwrap();
        let y1_0: [u64; 6] = p1[12..18].try_into().unwrap();
        let y1_1: [u64; 6] = p1[18..24].try_into().unwrap();
        if !lt(&x1_0, &P) || !lt(&x1_1, &P) || !lt(&y1_0, &P) || !lt(&y1_1, &P) {
            return Err(G2_ADD_ERR_NOT_IN_FIELD);
        }
        if !is_on_curve_twist_bls12_381(
            p1,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(G2_ADD_ERR_NOT_ON_CURVE);
        }
        return Ok(*p1);
    }

    // Both points are non-identity, validate both
    let x1_0: [u64; 6] = p1[0..6].try_into().unwrap();
    let x1_1: [u64; 6] = p1[6..12].try_into().unwrap();
    let y1_0: [u64; 6] = p1[12..18].try_into().unwrap();
    let y1_1: [u64; 6] = p1[18..24].try_into().unwrap();
    if !lt(&x1_0, &P) || !lt(&x1_1, &P) || !lt(&y1_0, &P) || !lt(&y1_1, &P) {
        return Err(G2_ADD_ERR_NOT_IN_FIELD);
    }
    if !is_on_curve_twist_bls12_381(
        p1,
        #[cfg(feature = "hints")]
        hints,
    ) {
        return Err(G2_ADD_ERR_NOT_ON_CURVE);
    }

    let x2_0: [u64; 6] = p2[0..6].try_into().unwrap();
    let x2_1: [u64; 6] = p2[6..12].try_into().unwrap();
    let y2_0: [u64; 6] = p2[12..18].try_into().unwrap();
    let y2_1: [u64; 6] = p2[18..24].try_into().unwrap();
    if !lt(&x2_0, &P) || !lt(&x2_1, &P) || !lt(&y2_0, &P) || !lt(&y2_1, &P) {
        return Err(G2_ADD_ERR_NOT_IN_FIELD);
    }
    if !is_on_curve_twist_bls12_381(
        p2,
        #[cfg(feature = "hints")]
        hints,
    ) {
        return Err(G2_ADD_ERR_NOT_ON_CURVE);
    }

    // Perform addition
    Ok(add_twist_bls12_381(
        p1,
        p2,
        #[cfg(feature = "hints")]
        hints,
    ))
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

/// Subtraction of two non-zero points `p1` and `p2` on the BLS12-381 curve
pub fn sub_twist_bls12_381(
    p1: &[u64; 24],
    p2: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let x2: [u64; 12] = p2[0..12].try_into().unwrap();
    let y2: [u64; 12] = p2[12..24].try_into().unwrap();

    // P1 - P2 = P1 + (-P2)
    let y2_neg = neg_fp2_bls12_381(
        &y2,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut p2_neg = [0u64; 24];
    p2_neg[0..12].copy_from_slice(&x2);
    p2_neg[12..24].copy_from_slice(&y2_neg);

    add_twist_bls12_381(
        p1,
        &p2_neg,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Subtraction of two points `p1` and `p2` on the BLS12-381 curve
pub fn sub_complete_twist_bls12_381(
    p1: &[u64; 24],
    p2: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let p1_is_inf = *p1 == G2_IDENTITY;
    let p2_is_inf = *p2 == G2_IDENTITY;

    // Handle identity cases
    if p1_is_inf && p2_is_inf {
        // O - O = O
        return G2_IDENTITY;
    } else if p1_is_inf {
        // O - P2 = -P2
        return neg_twist_bls12_381(
            p2,
            #[cfg(feature = "hints")]
            hints,
        );
    } else if p2_is_inf {
        // P1 - O = P1
        return *p1;
    }

    // Perform regular subtraction: P1 - P2 = P1 + (-P2)
    sub_twist_bls12_381(
        p1,
        p2,
        #[cfg(feature = "hints")]
        hints,
    )
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
    k: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0] => {
            // Return 𝒪
            return G2_IDENTITY;
        }
        [1, 0, 0, 0] => {
            // Return p
            return *p;
        }
        [2, 0, 0, 0] => {
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
    let mut q = *p;
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

/// Multi-Scalar Multiplication (MSM) for BLS12-381 G2 points
/// It computes k1·P1 + k2·P2 + ... + kn·Pn
// TODO: This is a naive implementation, one can improve it by using, e.g., a windowed strategies!
pub fn msm_complete_twist_bls12_381(
    points: &[[u64; 24]],
    scalars: &[[u64; 4]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 24], u8> {
    debug_assert_eq!(points.len(), scalars.len());

    let mut acc = G2_IDENTITY;
    let mut acc_is_inf = true;

    for (point, scalar) in points.iter().zip(scalars.iter()) {
        // Skip infinity points
        if *point == G2_IDENTITY {
            continue;
        }

        // Reduce the scalar modulo the group order, and skip if the result is zero
        let scalar = reduce_fr_bls12_381(
            scalar,
            #[cfg(feature = "hints")]
            hints,
        );
        if is_zero(&scalar) {
            continue;
        }

        // Verify point coordinates are in the field
        let x_0: [u64; 6] = point[0..6].try_into().unwrap();
        let x_1: [u64; 6] = point[6..12].try_into().unwrap();
        let y_0: [u64; 6] = point[12..18].try_into().unwrap();
        let y_1: [u64; 6] = point[18..24].try_into().unwrap();
        if !lt(&x_0, &P) || !lt(&x_1, &P) || !lt(&y_0, &P) || !lt(&y_1, &P) {
            return Err(G2_MSM_ERR_NOT_IN_FIELD);
        }

        // Verify point is on curve
        if !is_on_curve_twist_bls12_381(
            point,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(G2_MSM_ERR_NOT_ON_CURVE);
        }

        // Verify point is in subgroup (required for MSM per EIP-2537)
        if !is_on_subgroup_twist_bls12_381(
            point,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(G2_MSM_ERR_NOT_IN_SUBGROUP);
        }

        // Compute P * k
        let product = scalar_mul_twist_bls12_381(
            point,
            &scalar,
            #[cfg(feature = "hints")]
            hints,
        );

        // Skip if product is infinity
        if product == G2_IDENTITY {
            continue;
        }

        // Add to accumulator
        if acc_is_inf {
            acc = product;
            acc_is_inf = false;
        } else {
            acc = add_twist_bls12_381(
                &acc,
                &product,
                #[cfg(feature = "hints")]
                hints,
            );
            acc_is_inf = acc == G2_IDENTITY;
        }
    }

    Ok(acc)
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

/// G2 point addition for uncompressed 192-byte points (big-endian format)
///
/// Input format: 192 bytes per point = 96 bytes x-coordinate (Fp2) + 96 bytes y-coordinate (Fp2)
/// Each Fp2 element: 48 bytes imaginary + 48 bytes real (big-endian)
/// Output format: Same as input
///
/// ### Safety
/// - `a` must point to a valid `[u8; 192]` for the first input point
/// - `b` must point to a valid `[u8; 192]` for the second input point
/// - `ret` must point to a valid `[u8; 192]` for the output
///
/// Returns:
/// - [G2_ADD_SUCCESS] = success (regular point)
/// - [G2_ADD_SUCCESS_INFINITY] = success (point at infinity)
/// - [G2_ADD_ERR_NOT_IN_FIELD] = error (at least one point coordinate not in field)
/// - [G2_ADD_ERR_NOT_ON_CURVE] = error (at least one point not on curve)
#[inline]
pub(crate) unsafe fn bls12_381_g2_add_c(
    ret: *mut u8,
    a: *const u8,
    b: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a_bytes: &[u8; 192] = &*(a as *const [u8; 192]);
    let b_bytes: &[u8; 192] = &*(b as *const [u8; 192]);
    let ret_bytes: &mut [u8; 192] = &mut *(ret as *mut [u8; 192]);

    // Parse points
    let a_u64 = g2_bytes_be_to_u64_le_bls12_381(a_bytes);
    let b_u64 = g2_bytes_be_to_u64_le_bls12_381(b_bytes);

    // Perform addition
    let result = match add_complete_twist_bls12_381(
        &a_u64,
        &b_u64,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Encode result
    if result == G2_IDENTITY {
        G2_ADD_SUCCESS_INFINITY
    } else {
        g2_u64_le_to_bytes_be_bls12_381(&result, ret_bytes);
        G2_ADD_SUCCESS
    }
}

/// G2 Multi-Scalar Multiplication (MSM) for uncompressed points (big-endian format)
///
/// Input format per pair: 224 bytes = 192 bytes G2 point + 32 bytes scalar (big-endian)
/// Output format: 192 bytes G2 point
///
/// ### Safety
/// - `pairs` must point to an array of `num_pairs * 224` bytes
/// - `ret` must point to a valid `[u8; 192]` for the output
///
/// Returns:
/// - [G2_MSM_SUCCESS] = success (regular point)
/// - [G2_MSM_SUCCESS_INFINITY] = success (point at infinity)
/// - [G2_MSM_ERR_NOT_IN_FIELD] = error (at least one point coordinate not in field)
/// - [G2_MSM_ERR_NOT_ON_CURVE] = error (at least one point not on curve)
/// - [G2_MSM_ERR_NOT_IN_SUBGROUP] = error (at least one point not in subgroup)
#[inline]
pub(crate) unsafe fn bls12_381_g2_msm_c(
    ret: *mut u8,
    pairs: *const u8,
    num_pairs: usize,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let ret_bytes: &mut [u8; 192] = &mut *(ret as *mut [u8; 192]);

    // Parse all pairs
    let mut points = Vec::with_capacity(num_pairs);
    let mut scalars = Vec::with_capacity(num_pairs);
    for i in 0..num_pairs {
        let pair_ptr = pairs.add(i * 224);
        let point_bytes: &[u8; 192] = &*(pair_ptr as *const [u8; 192]);
        let scalar_bytes: &[u8; 32] = &*(pair_ptr.add(192) as *const [u8; 32]);

        // Parse point and scalar
        let point_u64 = g2_bytes_be_to_u64_le_bls12_381(point_bytes);
        let scalar_u64 = scalar_bytes_be_to_u64_le_bls12_381(scalar_bytes);

        points.push(point_u64);
        scalars.push(scalar_u64);
    }

    // Perform MSM with validation
    let result = match msm_complete_twist_bls12_381(
        &points,
        &scalars,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Encode result
    if result == G2_IDENTITY {
        G2_MSM_SUCCESS_INFINITY
    } else {
        g2_u64_le_to_bytes_be_bls12_381(&result, ret_bytes);
        G2_MSM_SUCCESS
    }
}

/// Convert 192-byte big-endian G2 point to [u64; 24] little-endian
pub fn g2_bytes_be_to_u64_le_bls12_381(bytes: &[u8; 192]) -> [u64; 24] {
    let mut result = [0u64; 24];

    // x_r (bytes 0-47) -> result[0..6]
    for i in 0..6 {
        for j in 0..8 {
            result[5 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // x_i (bytes 48-95) -> result[6..12]
    for i in 0..6 {
        for j in 0..8 {
            result[11 - i] |= (bytes[48 + i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // y_r (bytes 96-143) -> result[12..18]
    for i in 0..6 {
        for j in 0..8 {
            result[17 - i] |= (bytes[96 + i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // y_i (bytes 144-191) -> result[18..24]
    for i in 0..6 {
        for j in 0..8 {
            result[23 - i] |= (bytes[144 + i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    result
}

/// Convert [u64; 24] little-endian G2 point to 192-byte big-endian
pub fn g2_u64_le_to_bytes_be_bls12_381(limbs: &[u64; 24], bytes: &mut [u8; 192]) {
    // x_r (limbs[0..6]) -> bytes 0-47
    for i in 0..6 {
        let limb = limbs[5 - i];
        for j in 0..8 {
            bytes[i * 8 + j] = ((limb >> (8 * (7 - j))) & 0xFF) as u8;
        }
    }

    // x_i (limbs[6..12]) -> bytes 48-95
    for i in 0..6 {
        let limb = limbs[11 - i];
        for j in 0..8 {
            bytes[48 + i * 8 + j] = ((limb >> (8 * (7 - j))) & 0xFF) as u8;
        }
    }

    // y_r (limbs[12..18]) -> bytes 96-143
    for i in 0..6 {
        let limb = limbs[17 - i];
        for j in 0..8 {
            bytes[96 + i * 8 + j] = ((limb >> (8 * (7 - j))) & 0xFF) as u8;
        }
    }

    // y_i (limbs[18..24]) -> bytes 144-191
    for i in 0..6 {
        let limb = limbs[23 - i];
        for j in 0..8 {
            bytes[144 + i * 8 + j] = ((limb >> (8 * (7 - j))) & 0xFF) as u8;
        }
    }
}
