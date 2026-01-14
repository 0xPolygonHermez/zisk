//! Pairing over BN254

use crate::zisklib::lib::utils::gt;

use super::{
    constants::{IDENTITY_G1, IDENTITY_G2, P_MINUS_ONE},
    curve::is_on_curve_bn254,
    final_exp::final_exp_bn254,
    miller_loop::{miller_loop_batch_bn254, miller_loop_bn254},
    twist::{is_on_curve_twist_bn254, is_on_subgroup_twist_bn254},
};

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
    if *p == IDENTITY_G1 || *q == IDENTITY_G2 {
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
        if *p == IDENTITY_G1 || *q == IDENTITY_G2 {
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

/// # Safety
/// - `g1_ptr` must point to a contiguous array of `num_points` G1 affine points,
///   each being `[u64; 8]` (64 bytes per point).
/// - `g2_ptr` must point to a contiguous array of `num_points` G2 twist affine points,
///   each being `[u64; 16]` (128 bytes per point).
/// - `out_ptr` must point to a valid `[u64; 48]` (384 bytes) writable buffer for the GT result.
/// - `num_points` must correctly reflect the number of points in both arrays.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_pairing_batch_bn254_c")]
pub unsafe extern "C" fn pairing_batch_bn254_c(
    g1_ptr: *const u64,
    g2_ptr: *const u64,
    num_points: usize,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let g1_slice = core::slice::from_raw_parts(g1_ptr as *const [u64; 8], num_points);
    let g2_slice = core::slice::from_raw_parts(g2_ptr as *const [u64; 16], num_points);
    let result = pairing_batch_bn254(
        g1_slice,
        g2_slice,
        #[cfg(feature = "hints")]
        hints,
    );

    out_ptr.copy_from_nonoverlapping(result.as_ptr(), 48);
}
