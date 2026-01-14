//! Pairing over BLS12-381 curve

use crate::zisklib::lib::utils::gt;

use super::{
    constants::{IDENTITY_G1, IDENTITY_G2, P_MINUS_ONE},
    curve::{is_on_curve_bls12_381, is_on_subgroup_bls12_381, neg_bls12_381},
    final_exp::final_exp_bls12_381,
    miller_loop::{miller_loop_batch_bls12_381, miller_loop_bls12_381},
    twist::{is_on_curve_twist_bls12_381, is_on_subgroup_twist_bls12_381},
};

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
    if *p == IDENTITY_G1 || *q == IDENTITY_G2 {
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

/// Computes the optimal Ate pairing for a batch of G1 and G2 points over the BN254 curve
/// and multiplies the results together, i.e.:
///     e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) ∈ GT
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

    // Miller loop and multiplication
    let mut g1_points_ml = Vec::with_capacity(n);
    let mut g2_points_ml = Vec::with_capacity(n);
    for (p, q) in g1_points.iter().zip(g2_points.iter()) {
        // If p = 𝒪 or q = 𝒪 => MillerLoop(P, 𝒪) = MillerLoop(𝒪, Q) = 1; we can skip
        if *p != IDENTITY_G1 && *q != IDENTITY_G2 {
            g1_points_ml.push(*p);
            g2_points_ml.push(*q);
        }
    }

    if g1_points_ml.is_empty() {
        // If all pairing computations were skipped, return 1
        let mut one = [0; 72];
        one[0] = 1;
        return one;
    }

    // Miller loop
    let miller_loop = miller_loop_batch_bls12_381(
        &g1_points_ml,
        &g2_points_ml,
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

/// C-compatible wrapper for pairing_verify_bls12_381
///
/// # Safety
/// - All pointers must be valid and properly aligned
/// - `p1` and `p2` must point to at least 12 u64s each
/// - `q1` and `q2` must point to at least 24 u64s each
///
/// Returns 1 if e(P₁, Q₁) == e(P₂, Q₂), 0 otherwise
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_pairing_verify_bls12_381_c")]
pub unsafe extern "C" fn pairing_verify_bls12_381_c(
    p1_ptr: *const u64,
    q1_ptr: *const u64,
    p2_ptr: *const u64,
    q2_ptr: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p1: &[u64; 12] = &*(p1_ptr as *const [u64; 12]);
    let q1: &[u64; 24] = &*(q1_ptr as *const [u64; 24]);
    let p2: &[u64; 12] = &*(p2_ptr as *const [u64; 12]);
    let q2: &[u64; 24] = &*(q2_ptr as *const [u64; 24]);

    // Treat P₁,Q₁,P₂,Q₂ == 𝒪 at first, as this is a common case
    // e(P₁, 𝒪) == e(P₂, Q₂) <--> P₂ == 𝒪 || Q₂ == 𝒪
    // e(𝒪, Q₁) == e(P₂, Q₂) <--> P₂ == 𝒪 || Q₂ == 𝒪
    if *p1 == IDENTITY_G1 || *q1 == IDENTITY_G2 {
        return *p2 == IDENTITY_G1 || *q2 == IDENTITY_G2;
    } else if *p2 == IDENTITY_G1 || *q2 == IDENTITY_G2 {
        return false;
    }

    // Checking e(P1, Q1) == e(P2, Q2) is equivalent to checking e(P1, Q1) * e(-P2, Q2) == 1
    let p2_neg = neg_bls12_381(
        p2,
        #[cfg(feature = "hints")]
        hints,
    );
    let pairing_result = pairing_batch_bls12_381(
        &[*p1, p2_neg],
        &[*q1, *q2],
        #[cfg(feature = "hints")]
        hints,
    );

    let one = {
        let mut one = [0; 72];
        one[0] = 1;
        one
    };
    pairing_result == one
}
