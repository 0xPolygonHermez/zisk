use crate::zisklib::{eq, is_zero, lt};

use super::{
    constants::{
        COFACTOR_G1, ISO_A_G1, ISO_A_G2, ISO_B_G1, ISO_B_G2, ISO_X_DEN_G1, ISO_X_DEN_G2,
        ISO_X_NUM_G1, ISO_X_NUM_G2, ISO_Y_DEN_G1, ISO_Y_DEN_G2, ISO_Y_NUM_G1, ISO_Y_NUM_G2,
        SWU_Z2_G1, SWU_Z_G1, SWU_Z_G2,
    },
    curve::{g1_u64_le_to_bytes_be_bls12_381, scalar_mul_bls12_381},
    fp::{
        add_fp_bls12_381, bytes_be_to_u64_le_fp_bls12_381, inv_fp_bls12_381, mul_fp_bls12_381,
        neg_fp_bls12_381, sgn0_fp_bls12_381, sqrt_fp_bls12_381, square_fp_bls12_381,
    },
    fp2::{
        add_fp2_bls12_381, bytes_be_to_u64_le_fp2_bls12_381, inv_fp2_bls12_381, mul_fp2_bls12_381,
        neg_fp2_bls12_381, sgn0_fp2_bls12_381, sqrt_fp2_bls12_381, square_fp2_bls12_381,
    },
    twist::{
        clear_cofactor_twist_bls12_381, g2_u64_le_to_bytes_be_bls12_381, scalar_mul_twist_bls12_381,
    },
};

/// Maps a field element to a point on the BLS12-381 G1 curve
pub fn map_to_curve_g1_bls12_381(
    u: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    // Step 1: Map to isogenous curve E' using simplified SWU
    let p_prime = map_to_curve_simple_swu_g1_bls12_381(
        u,
        #[cfg(feature = "hints")]
        hints,
    );

    // Step 2: Apply isogeny map from E' to E
    let p = isogeny_map_g1_bls12_381(
        &p_prime,
        #[cfg(feature = "hints")]
        hints,
    );

    // Step 3: Clear cofactor
    scalar_mul_bls12_381(
        &p,
        &COFACTOR_G1,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Maps a field element in Fp2 to a point on the BLS12-381 G2 curve
pub fn map_to_curve_g2_bls12_381(
    u: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    // Step 1: Map to isogenous curve E' using simplified SWU
    let p_prime = map_to_curve_simple_swu_g2_bls12_381(
        u,
        #[cfg(feature = "hints")]
        hints,
    );

    // Step 2: Apply isogeny map from E' to E
    let p = isogeny_map_g2_bls12_381(
        &p_prime,
        #[cfg(feature = "hints")]
        hints,
    );

    // Step 3: Clear cofactor
    clear_cofactor_twist_bls12_381(
        &p,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Maps a field element u ∈ Fp to a point on the isogenous curve E'
/// using the simplified Shallue-van de Woestijne-Ulas (SWU) method for AB != 0
fn map_to_curve_simple_swu_g1_bls12_381(
    u: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    // 1. tv1 = inv0(Z^2 * u^4 + Z * u^2)
    let u2 = square_fp_bls12_381(
        u,
        #[cfg(feature = "hints")]
        hints,
    );
    let u4 = square_fp_bls12_381(
        &u2,
        #[cfg(feature = "hints")]
        hints,
    );
    let z_u2 = mul_fp_bls12_381(
        &SWU_Z_G1,
        &u2,
        #[cfg(feature = "hints")]
        hints,
    );
    let z2_u4 = mul_fp_bls12_381(
        &SWU_Z2_G1,
        &u4,
        #[cfg(feature = "hints")]
        hints,
    );
    let tv1_denom = add_fp_bls12_381(
        &z2_u4,
        &z_u2,
        #[cfg(feature = "hints")]
        hints,
    );
    let tv1 = inv_fp_bls12_381(
        &tv1_denom,
        #[cfg(feature = "hints")]
        hints,
    );

    // 2. x1 = (-B / A) * (1 + tv1)
    let neg_b = neg_fp_bls12_381(
        &ISO_B_G1,
        #[cfg(feature = "hints")]
        hints,
    );
    let a_inv = inv_fp_bls12_381(
        &ISO_A_G1,
        #[cfg(feature = "hints")]
        hints,
    );
    let neg_b_over_a = mul_fp_bls12_381(
        &neg_b,
        &a_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    let one = [1u64, 0, 0, 0, 0, 0];
    let one_plus_tv1 = add_fp_bls12_381(
        &one,
        &tv1,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut x1 = mul_fp_bls12_381(
        &neg_b_over_a,
        &one_plus_tv1,
        #[cfg(feature = "hints")]
        hints,
    );

    // 3. If tv1 == 0, set x1 = B / (Z * A)
    if is_zero(&tv1) {
        let z_a = mul_fp_bls12_381(
            &SWU_Z_G1,
            &ISO_A_G1,
            #[cfg(feature = "hints")]
            hints,
        );
        let z_a_inv = inv_fp_bls12_381(
            &z_a,
            #[cfg(feature = "hints")]
            hints,
        );
        x1 = mul_fp_bls12_381(
            &ISO_B_G1,
            &z_a_inv,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // 4. gx1 = x1^3 + A * x1 + B
    let gx1 = compute_y2_iso_g1_bls12_381(
        &x1,
        #[cfg(feature = "hints")]
        hints,
    );

    // 5. x2 = Z * u^2 * x1 (computed lazily below if needed)

    // 6. gx2 = x2^3 + A * x2 + B  (computed lazily below if needed)

    // 7-8. Select x and y based on whether gx1 is square
    let (y1, gx1_is_qr) = sqrt_fp_bls12_381(
        &gx1,
        #[cfg(feature = "hints")]
        hints,
    );
    let (x, mut y) = if gx1_is_qr {
        (x1, y1)
    } else {
        let x2 = mul_fp_bls12_381(
            &z_u2,
            &x1,
            #[cfg(feature = "hints")]
            hints,
        );
        let gx2 = compute_y2_iso_g1_bls12_381(
            &x2,
            #[cfg(feature = "hints")]
            hints,
        );
        let (y2, _) = sqrt_fp_bls12_381(
            &gx2,
            #[cfg(feature = "hints")]
            hints,
        );
        (x2, y2)
    };

    // 9. If sgn0(u) != sgn0(y), set y = -y
    if sgn0_fp_bls12_381(u) != sgn0_fp_bls12_381(&y) {
        y = neg_fp_bls12_381(
            &y,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // Return point (x, y) on E'
    let mut point = [0u64; 12];
    point[0..6].copy_from_slice(&x);
    point[6..12].copy_from_slice(&y);
    point
}

/// Maps a field element u ∈ Fp2 to a point on the isogenous curve E'
/// using the simplified Shallue-van de Woestijne-Ulas (SWU) method for AB != 0
fn map_to_curve_simple_swu_g2_bls12_381(
    u: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    // 1. tv1 = inv0(Z^2 * u^4 + Z * u^2)
    let u2 = square_fp2_bls12_381(
        u,
        #[cfg(feature = "hints")]
        hints,
    );
    let u4 = square_fp2_bls12_381(
        &u2,
        #[cfg(feature = "hints")]
        hints,
    );
    let z_u2 = mul_fp2_bls12_381(
        &SWU_Z_G2,
        &u2,
        #[cfg(feature = "hints")]
        hints,
    );
    let z2 = square_fp2_bls12_381(
        &SWU_Z_G2,
        #[cfg(feature = "hints")]
        hints,
    );
    let z2_u4 = mul_fp2_bls12_381(
        &z2,
        &u4,
        #[cfg(feature = "hints")]
        hints,
    );
    let tv1_denom = add_fp2_bls12_381(
        &z2_u4,
        &z_u2,
        #[cfg(feature = "hints")]
        hints,
    );
    let tv1 = inv_fp2_bls12_381(
        &tv1_denom,
        #[cfg(feature = "hints")]
        hints,
    );

    // 2. x1 = (-B / A) * (1 + tv1)
    let neg_b = neg_fp2_bls12_381(
        &ISO_B_G2,
        #[cfg(feature = "hints")]
        hints,
    );
    let a_inv = inv_fp2_bls12_381(
        &ISO_A_G2,
        #[cfg(feature = "hints")]
        hints,
    );
    let neg_b_over_a = mul_fp2_bls12_381(
        &neg_b,
        &a_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    let one: [u64; 12] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let one_plus_tv1 = add_fp2_bls12_381(
        &one,
        &tv1,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut x1 = mul_fp2_bls12_381(
        &neg_b_over_a,
        &one_plus_tv1,
        #[cfg(feature = "hints")]
        hints,
    );

    // 3. If tv1 == 0, set x1 = B / (Z * A)
    if is_zero(&tv1) {
        let z_a = mul_fp2_bls12_381(
            &SWU_Z_G2,
            &ISO_A_G2,
            #[cfg(feature = "hints")]
            hints,
        );
        let z_a_inv = inv_fp2_bls12_381(
            &z_a,
            #[cfg(feature = "hints")]
            hints,
        );
        x1 = mul_fp2_bls12_381(
            &ISO_B_G2,
            &z_a_inv,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // 4. gx1 = x1^3 + A * x1 + B
    let gx1 = compute_y2_iso_g2_bls12_381(
        &x1,
        #[cfg(feature = "hints")]
        hints,
    );

    // 7-8. Select x and y based on whether gx1 is square
    let (y1, gx1_is_qr) = sqrt_fp2_bls12_381(
        &gx1,
        #[cfg(feature = "hints")]
        hints,
    );
    let (x, mut y) = if gx1_is_qr {
        (x1, y1)
    } else {
        // 5. x2 = Z * u^2 * x1
        let x2 = mul_fp2_bls12_381(
            &z_u2,
            &x1,
            #[cfg(feature = "hints")]
            hints,
        );
        // 6. gx2 = x2^3 + A * x2 + B
        let gx2 = compute_y2_iso_g2_bls12_381(
            &x2,
            #[cfg(feature = "hints")]
            hints,
        );
        let (y2, _) = sqrt_fp2_bls12_381(
            &gx2,
            #[cfg(feature = "hints")]
            hints,
        );
        (x2, y2)
    };

    // 9. If sgn0(u) != sgn0(y), set y = -y
    if sgn0_fp2_bls12_381(u) != sgn0_fp2_bls12_381(&y) {
        y = neg_fp2_bls12_381(
            &y,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // Return point (x, y) on E'
    let mut point = [0u64; 24];
    point[0..12].copy_from_slice(&x);
    point[12..24].copy_from_slice(&y);
    point
}

/// Compute y² = x³ + A'x + B' for the isogenous curve E' (G1)
fn compute_y2_iso_g1_bls12_381(
    x: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 6] {
    let x2 = square_fp_bls12_381(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x3 = mul_fp_bls12_381(
        &x2,
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let ax = mul_fp_bls12_381(
        &ISO_A_G1,
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x3_ax = add_fp_bls12_381(
        &x3,
        &ax,
        #[cfg(feature = "hints")]
        hints,
    );
    add_fp_bls12_381(
        &x3_ax,
        &ISO_B_G1,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Compute y² = x³ + A'x + B' for the isogenous curve E' (G2)
fn compute_y2_iso_g2_bls12_381(
    x: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let x2 = square_fp2_bls12_381(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x3 = mul_fp2_bls12_381(
        &x2,
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let ax = mul_fp2_bls12_381(
        &ISO_A_G2,
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x3_ax = add_fp2_bls12_381(
        &x3,
        &ax,
        #[cfg(feature = "hints")]
        hints,
    );
    add_fp2_bls12_381(
        &x3_ax,
        &ISO_B_G2,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Apply the 11-isogeny map from E' to E for G1
fn isogeny_map_g1_bls12_381(
    p: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let x: [u64; 6] = p[0..6].try_into().unwrap();
    let y: [u64; 6] = p[6..12].try_into().unwrap();

    // Compute x-coordinate: x_num / x_den
    let x_num = eval_poly_fp(
        &ISO_X_NUM_G1,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_den = eval_poly_fp(
        &ISO_X_DEN_G1,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_den_inv = inv_fp_bls12_381(
        &x_den,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_out = mul_fp_bls12_381(
        &x_num,
        &x_den_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute y-coordinate: y' * y_num / y_den
    let y_num = eval_poly_fp(
        &ISO_Y_NUM_G1,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_den = eval_poly_fp(
        &ISO_Y_DEN_G1,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_den_inv = inv_fp_bls12_381(
        &y_den,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_frac = mul_fp_bls12_381(
        &y_num,
        &y_den_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_out = mul_fp_bls12_381(
        &y,
        &y_frac,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&x_out);
    result[6..12].copy_from_slice(&y_out);
    result
}

/// Apply the 3-isogeny map from E' to E for G2
fn isogeny_map_g2_bls12_381(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 24] {
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();

    // Compute x-coordinate: x_num / x_den
    let x_num = eval_poly_fp2(
        &ISO_X_NUM_G2,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_den = eval_poly_fp2(
        &ISO_X_DEN_G2,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_den_inv = inv_fp2_bls12_381(
        &x_den,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_out = mul_fp2_bls12_381(
        &x_num,
        &x_den_inv,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute y-coordinate: y' * y_num / y_den
    let y_num = eval_poly_fp2(
        &ISO_Y_NUM_G2,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_den = eval_poly_fp2(
        &ISO_Y_DEN_G2,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_den_inv = inv_fp2_bls12_381(
        &y_den,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_frac = mul_fp2_bls12_381(
        &y_num,
        &y_den_inv,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_out = mul_fp2_bls12_381(
        &y,
        &y_frac,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0u64; 24];
    result[0..12].copy_from_slice(&x_out);
    result[12..24].copy_from_slice(&y_out);
    result
}

/// Evaluate a polynomial at x
fn eval_poly_fp<const N: usize>(
    coeffs: &[[u64; 6]; N],
    x: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 6] {
    // Use Horner's method
    let mut result = coeffs[N - 1];
    for i in (0..N - 1).rev() {
        result = mul_fp_bls12_381(
            &result,
            x,
            #[cfg(feature = "hints")]
            hints,
        );
        result = add_fp_bls12_381(
            &result,
            &coeffs[i],
            #[cfg(feature = "hints")]
            hints,
        );
    }
    result
}

/// Evaluate a polynomial at x over Fp2
fn eval_poly_fp2<const N: usize>(
    coeffs: &[[u64; 12]; N],
    x: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    // Use Horner's method
    let mut result = coeffs[N - 1];
    for i in (0..N - 1).rev() {
        result = mul_fp2_bls12_381(
            &result,
            x,
            #[cfg(feature = "hints")]
            hints,
        );
        result = add_fp2_bls12_381(
            &result,
            &coeffs[i],
            #[cfg(feature = "hints")]
            hints,
        );
    }
    result
}

/// BLS12-381 map Fp field element to G1 point
///
/// Input format: 48 bytes field element (big-endian)
/// Output format: 96 bytes G1 point (x || y big-endian)
///
/// ### Safety
/// - `fp` must point to a valid `[u8; 48]`
/// - `ret` must point to a valid `[u8; 96]` for the output
///
/// Returns:
/// - 0 = success
/// - 1 = error (input not in field)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_bls12_381_fp_to_g1_c")]
pub unsafe extern "C" fn bls12_381_fp_to_g1_c(
    ret: *mut u8,
    fp: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let fp_bytes: &[u8; 48] = &*(fp as *const [u8; 48]);
    let ret_bytes: &mut [u8; 96] = &mut *(ret as *mut [u8; 96]);

    // Parse field element
    let u = bytes_be_to_u64_le_fp_bls12_381(fp_bytes);

    // Map to curve
    let result = map_to_curve_g1_bls12_381(
        &u,
        #[cfg(feature = "hints")]
        hints,
    );

    // Encode result
    g1_u64_le_to_bytes_be_bls12_381(&result, ret_bytes);
    0
}

/// BLS12-381 map Fp2 field element to G2 point
///
/// Input format: 96 bytes Fp2 element (c0 || c1, each 48 bytes big-endian)
/// Output format: 192 bytes G2 point (x_r || x_i || y_r || y_i, each 48 bytes big-endian)
///
/// ### Safety
/// - `fp2` must point to a valid `[u8; 96]`
/// - `ret` must point to a valid `[u8; 192]` for the output
///
/// Returns:
/// - 0 = success
/// - 1 = error (input not in field)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_bls12_381_fp2_to_g2_c")]
pub unsafe extern "C" fn bls12_381_fp2_to_g2_c(
    ret: *mut u8,
    fp2: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let fp2_bytes: &[u8; 96] = &*(fp2 as *const [u8; 96]);
    let ret_bytes: &mut [u8; 192] = &mut *(ret as *mut [u8; 192]);

    // Parse Fp2 element
    let u = bytes_be_to_u64_le_fp2_bls12_381(fp2_bytes);

    // Map to curve
    let result = map_to_curve_g2_bls12_381(
        &u,
        #[cfg(feature = "hints")]
        hints,
    );

    // Encode result
    g2_u64_le_to_bytes_be_bls12_381(&result, ret_bytes);
    0
}
