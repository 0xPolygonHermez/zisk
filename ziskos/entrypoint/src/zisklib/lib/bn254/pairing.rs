//! Pairing over BN254

use crate::zisklib::lib::utils::{eq, is_one, lt};

use super::{
    constants::{G1_IDENTITY, G2_IDENTITY, P},
    curve::{g1_bytes_be_to_u64_le_bn254, is_on_curve_bn254},
    final_exp::final_exp_bn254,
    miller_loop::{miller_loop_batch_bn254, miller_loop_bn254},
    twist::{g2_bytes_be_to_u64_le_bn254, is_on_curve_twist_bn254, is_on_subgroup_twist_bn254},
};

/// Pairing check result codes
const PAIRING_CHECK_SUCCESS: u8 = 0;
const PAIRING_CHECK_FAILED: u8 = 1;
const PAIRING_CHECK_ERR_G1_INVALID: u8 = 2;
const PAIRING_CHECK_ERR_G1_NOT_ON_CURVE: u8 = 3;
const PAIRING_CHECK_ERR_G2_INVALID: u8 = 4;
const PAIRING_CHECK_ERR_G2_NOT_ON_CURVE: u8 = 5;
const PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP: u8 = 6;

/// Optimal Ate Pairing e: G1 x G2 -> GT over the BN254 curve
/// where G1 = E(Fp)[r] = E(Fp), G2 = E'(Fp2)[r] and GT = μ_r (the r-th roots of unity over Fp12*
/// the involved curves are E/Fp: y² = x³ + 3 and E'/Fp2: y² = x³ + 3/(9+u)
///  pairingBN254:
///          input: P ∈ G1 and Q ∈ G2
///          output: e(P,Q) ∈ GT
///
pub fn pairing_bn254(
    p: &[u64; 8],
    q: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 48] {
    // Is p = 𝒪?
    if *p == G1_IDENTITY || *q == G2_IDENTITY {
        // e(P, 𝒪) = e(𝒪, Q) = 1;
        let mut one = [0; 48];
        one[0] = 1;
        return one;
    }

    // Miller loop
    let miller_loop = miller_loop_bn254(
        p,
        q,
        #[cfg(feature = "hints")]
        hints,
    );

    // Final exponentiation
    final_exp_bn254(
        &miller_loop,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Computes the optimal Ate pairing for a batch of G1 and G2 points over the BN254 curve
/// and multiplies the results together, i.e.:
///     e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) ∈ GT
pub fn pairing_batch_bn254(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 48] {
    // Since each e(Pi, Qi) := FinalExp(MillerLoop(Pi, Qi))
    // We have:
    //  e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) = FinalExp(MillerLoop(P₁, Q₁) · MillerLoop(P₂, Q₂) · ... · MillerLoop(Pₙ, Qₙ))
    // We can compute the Miller loop for each pair, multiplying the results together
    // and then just do the final exponentiation once at the end.

    let num_points = g1_points.len();
    assert_eq!(num_points, g2_points.len(), "Number of G1 and G2 points must be equal");

    // Miller loop and multiplication
    let mut g1_points_ml = Vec::with_capacity(num_points);
    let mut g2_points_ml = Vec::with_capacity(num_points);
    for (p, q) in g1_points.iter().zip(g2_points.iter()) {
        // Is p = 𝒪 or q = 𝒪?
        if *p == G1_IDENTITY || *q == G2_IDENTITY {
            // MillerLoop(P, 𝒪) = MillerLoop(𝒪, Q) = 1; we can skip
            continue;
        }

        g1_points_ml.push(*p);
        g2_points_ml.push(*q);
    }

    if g1_points_ml.is_empty() {
        // If all pairing computations were skipped, return 1
        let mut one = [0; 48];
        one[0] = 1;
        return one;
    }

    // Compute the Miller loop for the batch
    let miller_loop = miller_loop_batch_bn254(
        &g1_points_ml,
        &g2_points_ml,
        #[cfg(feature = "hints")]
        hints,
    );

    // Final exponentiation
    final_exp_bn254(
        &miller_loop,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// BN254 pairing check with validation.
///
/// Validates all points have canonical field elements, are on curve, and G2 points are in subgroup.
///
/// # Arguments
/// * `g1_points` - Slice of G1 points as [u64; 8]
/// * `g2_points` - Slice of G2 points as [u64; 16]
///
/// # Returns
/// * `Ok(true)` - Pairing check passed (product of pairings == 1)
/// * `Ok(false)` - Pairing check failed (product of pairings != 1)
/// * `Err(PAIRING_CHECK_ERR_G1_INVALID)` - G1 field element not canonical (>= P)
/// * `Err(PAIRING_CHECK_ERR_G1_NOT_ON_CURVE)` - G1 point not on curve
/// * `Err(PAIRING_CHECK_ERR_G2_INVALID)` - G2 field element not canonical (>= P)
/// * `Err(PAIRING_CHECK_ERR_G2_NOT_ON_CURVE)` - G2 point not on twist curve
/// * `Err(PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP)` - G2 point not in subgroup
pub fn pairing_check_bn254(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<bool, u8> {
    assert_eq!(g1_points.len(), g2_points.len(), "Number of G1 and G2 points must be equal");

    // Collect valid pairs
    let mut g1_valid = Vec::with_capacity(g1_points.len());
    let mut g2_valid = Vec::with_capacity(g2_points.len());
    for (g1, g2) in g1_points.iter().zip(g2_points.iter()) {
        let g1_is_inf = eq(g1, &G1_IDENTITY);
        let g2_is_inf = eq(g2, &G2_IDENTITY);

        // If p = 𝒪 or q = 𝒪 => MillerLoop(P, 𝒪) = MillerLoop(𝒪, Q) = 1; we can skip
        if g2_is_inf {
            if !g1_is_inf
                && !is_on_curve_bn254(
                    g1,
                    #[cfg(feature = "hints")]
                    hints,
                )
            {
                return Err(PAIRING_CHECK_ERR_G1_NOT_ON_CURVE);
            }
            continue;
        }

        if g1_is_inf {
            if !is_on_curve_twist_bn254(
                g2,
                #[cfg(feature = "hints")]
                hints,
            ) {
                return Err(PAIRING_CHECK_ERR_G2_NOT_ON_CURVE);
            }
            if !is_on_subgroup_twist_bn254(
                g2,
                #[cfg(feature = "hints")]
                hints,
            ) {
                return Err(PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP);
            }
            continue;
        }

        // Validate G1 point field elements
        let x1: [u64; 4] = g1[0..4].try_into().unwrap();
        let y1: [u64; 4] = g1[4..8].try_into().unwrap();
        if !lt(&x1, &P) || !lt(&y1, &P) {
            return Err(PAIRING_CHECK_ERR_G1_INVALID);
        }

        // Verify G1 point is on curve
        if !is_on_curve_bn254(
            g1,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(PAIRING_CHECK_ERR_G1_NOT_ON_CURVE);
        }

        // Validate G2 point field elements
        let x2_r: [u64; 4] = g2[0..4].try_into().unwrap();
        let x2_i: [u64; 4] = g2[4..8].try_into().unwrap();
        let y2_r: [u64; 4] = g2[8..12].try_into().unwrap();
        let y2_i: [u64; 4] = g2[12..16].try_into().unwrap();
        if !lt(&x2_r, &P) || !lt(&x2_i, &P) || !lt(&y2_r, &P) || !lt(&y2_i, &P) {
            return Err(PAIRING_CHECK_ERR_G2_INVALID);
        }

        // Verify G2 point is on twist curve
        if !is_on_curve_twist_bn254(
            g2,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(PAIRING_CHECK_ERR_G2_NOT_ON_CURVE);
        }

        // Verify G2 point is in subgroup
        if !is_on_subgroup_twist_bn254(
            g2,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return Err(PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP);
        }

        g1_valid.push(*g1);
        g2_valid.push(*g2);
    }

    // If all pairs were skipped, result is 1
    if g1_valid.is_empty() {
        return Ok(true);
    }

    // Compute batch pairing and check if result is 1
    Ok(is_one(&pairing_batch_bn254(
        &g1_valid,
        &g2_valid,
        #[cfg(feature = "hints")]
        hints,
    )))
}

/// BN254 pairing check with big-endian byte format
///
/// # Safety
/// - `pairs` must point to an array of `num_pairs * 192` bytes
///   Each pair is: 64 bytes G1 point + 128 bytes G2 point
///
/// # Returns
/// - 0 = pairing check passed
/// - 1 = pairing check failed
/// - 2 = G1 field element invalid
/// - 3 = G1 point not on curve
/// - 4 = G2 field element invalid
/// - 5 = G2 point not on curve
/// - 6 = G2 point not in subgroup
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_bn254_pairing_check_c")]
pub unsafe extern "C" fn bn254_pairing_check_c(
    pairs: *const u8,
    num_pairs: usize,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    // Parse all pairs
    let mut g1_points: Vec<[u64; 8]> = Vec::with_capacity(num_pairs);
    let mut g2_points: Vec<[u64; 16]> = Vec::with_capacity(num_pairs);

    for i in 0..num_pairs {
        let pair_ptr = pairs.add(i * 192);

        let g1_bytes: &[u8; 64] = &*(pair_ptr as *const [u8; 64]);
        let g2_bytes: &[u8; 128] = &*(pair_ptr.add(64) as *const [u8; 128]);

        g1_points.push(g1_bytes_be_to_u64_le_bn254(g1_bytes));
        g2_points.push(g2_bytes_be_to_u64_le_bn254(g2_bytes));
    }

    // Perform pairing check with validation
    match pairing_check_bn254(
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
