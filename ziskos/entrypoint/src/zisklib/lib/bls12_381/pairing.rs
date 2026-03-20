//! Pairing over BLS12-381 curve

use crate::zisklib::lib::utils::{eq, is_one, lt};

use super::{
    constants::{G1_IDENTITY, G2_IDENTITY, P, P_MINUS_ONE},
    curve::{
        g1_bytes_be_to_u64_le_bls12_381, is_on_curve_bls12_381, is_on_subgroup_bls12_381,
        neg_bls12_381,
    },
    final_exp::final_exp_bls12_381,
    miller_loop::{miller_loop_batch_bls12_381, miller_loop_bls12_381},
    twist::{
        g2_bytes_be_to_u64_le_bls12_381, is_on_curve_twist_bls12_381,
        is_on_subgroup_twist_bls12_381,
    },
};

/// Pairing check result codes
const PAIRING_CHECK_SUCCESS: u8 = 0;
const PAIRING_CHECK_FAILED: u8 = 1;
const PAIRING_CHECK_ERR_G1_NOT_IN_FIELD: u8 = 2;
const PAIRING_CHECK_ERR_G1_NOT_ON_CURVE: u8 = 3;
const PAIRING_CHECK_ERR_G1_NOT_IN_SUBGROUP: u8 = 4;
const PAIRING_CHECK_ERR_G2_NOT_IN_FIELD: u8 = 5;
const PAIRING_CHECK_ERR_G2_NOT_ON_CURVE: u8 = 6;
const PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP: u8 = 7;

/// Optimal Ate Pairing e: G1 x G2 -> GT over the BLS12-381 curve
/// where G1 = E(Fp)[r] = E(Fp), G2 = E'(Fp2)[r] and GT = μ_r (the r-th roots of unity over Fp12*)
/// the involved curves are E/Fp: y² = x³ + 4 and E'/Fp2: y² = x³ + 4·(1+u)
///  pairingBLS12-381:
///          input: P ∈ G1 and Q ∈ G2
///          output: e(P,Q) ∈ GT
pub fn pairing_bls12_381(
    p: &[u64; 12],
    q: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 72] {
    // e(P, 𝒪) = e(𝒪, Q) = 1;
    if *p == G1_IDENTITY || *q == G2_IDENTITY {
        let mut one = [0; 72];
        one[0] = 1;
        return one;
    }

    // Miller loop
    let miller_loop = miller_loop_bls12_381(
        p,
        q,
        #[cfg(feature = "hints")]
        hints,
    );

    // Final exponentiation
    final_exp_bls12_381(
        &miller_loop,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Computes the optimal Ate pairing for a batch of G1 and G2 points over the BLS12-381 curve
/// and multiplies the results together:
///     e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) ∈ GT
///
/// Assumes all points are non-infinity and already validated (on curve and in subgroup).
pub fn pairing_batch_bls12_381(
    g1_points: &[[u64; 12]],
    g2_points: &[[u64; 24]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 72] {
    // Since each e(Pi, Qi) := FinalExp(MillerLoop(Pi, Qi))
    // We have:
    //  e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) = FinalExp(MillerLoop(P₁, Q₁) · MillerLoop(P₂, Q₂) · ... · MillerLoop(Pₙ, Qₙ))
    // We can compute the Miller loop for each pair, multiplying the results together
    // and then just do the final exponentiation once at the end.

    let n = g1_points.len();
    assert_eq!(n, g2_points.len(), "Number of G1 and G2 points must be equal");

    if n == 0 {
        // Empty input returns 1
        let mut one = [0; 72];
        one[0] = 1;
        return one;
    }

    let miller_loop = miller_loop_batch_bls12_381(
        g1_points,
        g2_points,
        #[cfg(feature = "hints")]
        hints,
    );

    final_exp_bls12_381(
        &miller_loop,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Pairing check with validation
pub fn pairing_check_bls12_381(
    g1_points: &[[u64; 12]],
    g2_points: &[[u64; 24]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<bool, u8> {
    assert_eq!(g1_points.len(), g2_points.len(), "Number of G1 and G2 points must be equal");

    // Collect valid pairs
    let mut valid_g1: Vec<[u64; 12]> = Vec::with_capacity(g1_points.len());
    let mut valid_g2: Vec<[u64; 24]> = Vec::with_capacity(g2_points.len());
    for (g1, g2) in g1_points.iter().zip(g2_points.iter()) {
        let g1_is_inf = eq(g1, &G1_IDENTITY);
        let g2_is_inf = eq(g2, &G2_IDENTITY);

        if g1_is_inf && g2_is_inf {
            // If p = 𝒪 and q = 𝒪 => e(𝒪, 𝒪) = 1; we can skip
            continue;
        }

        // If q = 𝒪 => MillerLoop(P, 𝒪) = 1; we can skip
        if g2_is_inf {
            // Validate p1 field elements and curve membership
            let x1: [u64; 6] = g1[0..6].try_into().unwrap();
            let y1: [u64; 6] = g1[6..12].try_into().unwrap();
            if !lt(&x1, &P) || !lt(&y1, &P) {
                return Err(PAIRING_CHECK_ERR_G1_NOT_IN_FIELD);
            }
            if !is_on_curve_bls12_381(
                g1,
                #[cfg(feature = "hints")]
                hints,
            ) {
                return Err(PAIRING_CHECK_ERR_G1_NOT_ON_CURVE);
            }
            if !is_on_subgroup_bls12_381(
                g1,
                #[cfg(feature = "hints")]
                hints,
            ) {
                return Err(PAIRING_CHECK_ERR_G1_NOT_IN_SUBGROUP);
            }
            continue;
        }

        // If p = 𝒪 => MillerLoop(𝒪, Q) = 1; we can skip
        if g1_is_inf {
            // Validate p2 field elements and curve membership
            let x2_0: [u64; 6] = g2[0..6].try_into().unwrap();
            let x2_1: [u64; 6] = g2[6..12].try_into().unwrap();
            let y2_0: [u64; 6] = g2[12..18].try_into().unwrap();
            let y2_1: [u64; 6] = g2[18..24].try_into().unwrap();
            if !lt(&x2_0, &P) || !lt(&x2_1, &P) || !lt(&y2_0, &P) || !lt(&y2_1, &P) {
                return Err(PAIRING_CHECK_ERR_G2_NOT_IN_FIELD);
            }
            if !is_on_curve_twist_bls12_381(
                g2,
                #[cfg(feature = "hints")]
                hints,
            ) {
                return Err(PAIRING_CHECK_ERR_G2_NOT_ON_CURVE);
            }
            if !is_on_subgroup_twist_bls12_381(
                g2,
                #[cfg(feature = "hints")]
                hints,
            ) {
                return Err(PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP);
            }
            continue;
        }

        // Both points are non-identity, validate both
        let x1: [u64; 6] = g1[0..6].try_into().unwrap();
        let y1: [u64; 6] = g1[6..12].try_into().unwrap();
        if !lt(&x1, &P) || !lt(&y1, &P) {
            return Err(PAIRING_CHECK_ERR_G1_NOT_IN_FIELD);
        }
        if !is_on_curve_bls12_381(
            g1,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(PAIRING_CHECK_ERR_G1_NOT_ON_CURVE);
        }
        if !is_on_subgroup_bls12_381(
            g1,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(PAIRING_CHECK_ERR_G1_NOT_IN_SUBGROUP);
        }

        let x2_0: [u64; 6] = g2[0..6].try_into().unwrap();
        let x2_1: [u64; 6] = g2[6..12].try_into().unwrap();
        let y2_0: [u64; 6] = g2[12..18].try_into().unwrap();
        let y2_1: [u64; 6] = g2[18..24].try_into().unwrap();
        if !lt(&x2_0, &P) || !lt(&x2_1, &P) || !lt(&y2_0, &P) || !lt(&y2_1, &P) {
            return Err(PAIRING_CHECK_ERR_G2_NOT_IN_FIELD);
        }
        if !is_on_curve_twist_bls12_381(
            g2,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(PAIRING_CHECK_ERR_G2_NOT_ON_CURVE);
        }
        if !is_on_subgroup_twist_bls12_381(
            g2,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP);
        }

        valid_g1.push(*g1);
        valid_g2.push(*g2);
    }

    // If all pairs were skipped, result is 1
    if valid_g1.is_empty() {
        return Ok(true);
    }

    // Compute batch pairing and check if result is 1
    Ok(is_one(&pairing_batch_bls12_381(
        &valid_g1,
        &valid_g2,
        #[cfg(feature = "hints")]
        hints,
    )))
}

/// BLS12-381 pairing check for big-endian byte format.
///
/// # Input format
/// Per pair: 288 bytes = 96 bytes G1 point + 192 bytes G2 point (big-endian)
/// - G1 point: 48 bytes x + 48 bytes y
/// - G2 point: 48 bytes x_i + 48 bytes x_r + 48 bytes y_i + 48 bytes y_r
///
/// # Safety
/// `pairs` must point to an array of `num_pairs * 288` bytes
///
/// # Returns
/// - [PAIRING_CHECK_SUCCESS] = pairing check passed
/// - [PAIRING_CHECK_FAILED] = pairing check failed
/// - [PAIRING_CHECK_ERR_G1_NOT_IN_FIELD] = error (at least one G1 point coordinate not in field)
/// - [PAIRING_CHECK_ERR_G1_NOT_ON_CURVE] = error (at least one G1 point not on curve)
/// - [PAIRING_CHECK_ERR_G1_NOT_IN_SUBGROUP] = error (at least one G1 point not in subgroup)
/// - [PAIRING_CHECK_ERR_G2_NOT_IN_FIELD] = error (at least one G2 point coordinate not in field)
/// - [PAIRING_CHECK_ERR_G2_NOT_ON_CURVE] = error (at least one G2 point not on curve)
/// - [PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP] = error (at least one G2 point not in subgroup)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_bls12_381_pairing_check_c")]
pub unsafe extern "C" fn bls12_381_pairing_check_c(
    pairs: *const u8,
    num_pairs: usize,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    // Parse all pairs
    let mut g1_points: Vec<[u64; 12]> = Vec::with_capacity(num_pairs);
    let mut g2_points: Vec<[u64; 24]> = Vec::with_capacity(num_pairs);
    for i in 0..num_pairs {
        let pair_ptr = pairs.add(i * 288);

        let g1_bytes: &[u8; 96] = &*(pair_ptr as *const [u8; 96]);
        let g2_bytes: &[u8; 192] = &*(pair_ptr.add(96) as *const [u8; 192]);

        g1_points.push(g1_bytes_be_to_u64_le_bls12_381(g1_bytes));
        g2_points.push(g2_bytes_be_to_u64_le_bls12_381(g2_bytes));
    }

    match pairing_check_bls12_381(
        &g1_points,
        &g2_points,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(true) => PAIRING_CHECK_SUCCESS,
        Ok(false) => PAIRING_CHECK_FAILED,
        Err(code) => code,
    }
}
