//! Pairing over BN254

#[cfg(zisk_guest)]
use crate::alloc_extern::vec::Vec;

use crate::zisklib::{
    fcall_bn254_final_exp_witness,
    lib::utils::{eq, is_one, is_zero, lt, slice_from_ffi},
    BN254_FINAL_EXP_WITNESS_MAX_PAIRS,
};

use super::{
    constants::{G1_IDENTITY, G2_IDENTITY, P},
    curve::{g1_bytes_be_to_u64_le_bn254, is_on_curve_bn254},
    final_exp::{final_exp_bn254, miller_loop_witness_is_one_bn254},
    fp12::inv_fp12_bn254,
    miller_loop::{
        miller_loop_batch_bn254, miller_loop_batch_with_witness_bn254, miller_loop_bn254,
    },
    twist::{g2_bytes_be_to_u64_le_bn254, is_on_curve_twist_bn254, is_on_subgroup_twist_bn254},
};

/// Pairing check result codes
#[allow(dead_code)]
pub(crate) const PAIRING_CHECK_SUCCESS: u8 = 0;
#[allow(dead_code)]
pub(crate) const PAIRING_CHECK_FAILED: u8 = 1;
const PAIRING_CHECK_ERR_G1_NOT_CANONICAL: u8 = 2;
const PAIRING_CHECK_ERR_G1_NOT_ON_CURVE: u8 = 3;
const PAIRING_CHECK_ERR_G2_NOT_CANONICAL: u8 = 4;
const PAIRING_CHECK_ERR_G2_NOT_ON_CURVE: u8 = 5;
const PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP: u8 = 6;

/// Optimal Ate Pairing e: G1 x G2 -> GT over the BN254 curve
/// where G1 = E(Fp)[r] = E(Fp), G2 = E'(Fp2)[r] and GT = μ_r (the r-th roots of unity over Fp12*
/// the involved curves are E/Fp: y² = x³ + 3 and E'/Fp2: y² = x³ + 3/(9+u)
///  pairingBN254:
///          input: P ∈ G1 and Q ∈ G2
///          output: e(P,Q) ∈ GT
///
/// # Soundness
/// Both points must be on the corresponding subgroups, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
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
///
/// # Soundness
/// All points must be on the corresponding subgroups, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
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
    let (g1_points, g2_points) = non_identity_pairs_bn254(g1_points, g2_points);
    if g1_points.is_empty() {
        // If all pairing computations were skipped, return 1
        let mut one = [0; 48];
        one[0] = 1;
        return one;
    }

    let miller_loop = miller_loop_batch_bn254(
        &g1_points,
        &g2_points,
        #[cfg(feature = "hints")]
        hints,
    );
    final_exp_bn254(
        &miller_loop,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Checks e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) == 1, with a residue witness in place of the
/// final exponentiation when the pairs fit the witness fcall
///
/// # Soundness
/// All points must be on the corresponding subgroups and have **canonical** coordinates (`x, y < p`).
pub fn pairing_batch_check_bn254(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let (g1_points, g2_points) = non_identity_pairs_bn254(g1_points, g2_points);
    if g1_points.is_empty() {
        return true;
    }

    // The witness rides along the Miller loop when the pairs fit the fcall
    if g1_points.len() <= BN254_FINAL_EXP_WITNESS_MAX_PAIRS {
        let (c_inv, s) = fcall_bn254_final_exp_witness(
            &g1_points,
            &g2_points,
            #[cfg(feature = "hints")]
            hints,
        );
        // A zero c⁻¹ would wipe the loop: that hint takes the plain path
        if !is_zero(&c_inv) {
            // The loop multiplies by c at the -1 digits
            let c = inv_fp12_bn254(
                &c_inv,
                #[cfg(feature = "hints")]
                hints,
            );
            let f = miller_loop_batch_with_witness_bn254(
                &g1_points,
                &g2_points,
                Some((&c_inv, &c)),
                #[cfg(feature = "hints")]
                hints,
            );
            return miller_loop_witness_is_one_bn254(
                &f,
                &c_inv,
                &c,
                &s,
                #[cfg(feature = "hints")]
                hints,
            );
        }
    }

    // More pairs than the witness fcall takes, or no witness: the full final exponentiation
    is_one(&pairing_batch_bn254(
        &g1_points,
        &g2_points,
        #[cfg(feature = "hints")]
        hints,
    ))
}

/// The pairs with no identity point: MillerLoop(P, 𝒪) = MillerLoop(𝒪, Q) = 1.
fn non_identity_pairs_bn254(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
) -> (Vec<[u64; 8]>, Vec<[u64; 16]>) {
    assert_eq!(g1_points.len(), g2_points.len(), "Number of G1 and G2 points must be equal");
    g1_points
        .iter()
        .zip(g2_points)
        .filter(|(p, q)| **p != G1_IDENTITY && **q != G2_IDENTITY)
        .map(|(p, q)| (*p, *q))
        .unzip()
}

/// BN254 pairing check with validation
pub fn pairing_check_safe_bn254(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<bool, u8> {
    validate_pairing_points_bn254(
        g1_points,
        g2_points,
        #[cfg(feature = "hints")]
        hints,
    )?;

    // Check whether e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) == 1
    Ok(pairing_batch_check_bn254(
        g1_points,
        g2_points,
        #[cfg(feature = "hints")]
        hints,
    ))
}

/// BN254 batch pairing with validation: checks that every point has canonical coordinates,
/// lies on its curve and (for G2) in the prime-order subgroup, then returns
/// e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ) ∈ GT. Identity points are allowed.
pub fn pairing_batch_safe_bn254(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 48], u8> {
    validate_pairing_points_bn254(
        g1_points,
        g2_points,
        #[cfg(feature = "hints")]
        hints,
    )?;

    // Compute batch pairing e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ)
    Ok(pairing_batch_bn254(
        g1_points,
        g2_points,
        #[cfg(feature = "hints")]
        hints,
    ))
}

/// Checks that every point has canonical coordinates, lies on its curve and (for G2) in the
/// prime-order subgroup. Identity points are allowed.
fn validate_pairing_points_bn254(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<(), u8> {
    assert_eq!(g1_points.len(), g2_points.len(), "Number of G1 and G2 points must be equal");

    // Validate every point pair
    for (g1, g2) in g1_points.iter().zip(g2_points.iter()) {
        // Validate G1 point field elements
        let x1: [u64; 4] = g1[0..4].try_into().unwrap();
        let y1: [u64; 4] = g1[4..8].try_into().unwrap();
        if !lt(&x1, &P) || !lt(&y1, &P) {
            return Err(PAIRING_CHECK_ERR_G1_NOT_CANONICAL);
        }

        if !eq(g1, &G1_IDENTITY)
            && !is_on_curve_bn254(
                g1,
                #[cfg(feature = "hints")]
                hints,
            )
        {
            return Err(PAIRING_CHECK_ERR_G1_NOT_ON_CURVE);
        }

        // Validate G2 point field elements
        let x2_r: [u64; 4] = g2[0..4].try_into().unwrap();
        let x2_i: [u64; 4] = g2[4..8].try_into().unwrap();
        let y2_r: [u64; 4] = g2[8..12].try_into().unwrap();
        let y2_i: [u64; 4] = g2[12..16].try_into().unwrap();
        if !lt(&x2_r, &P) || !lt(&x2_i, &P) || !lt(&y2_r, &P) || !lt(&y2_i, &P) {
            return Err(PAIRING_CHECK_ERR_G2_NOT_CANONICAL);
        }

        if !eq(g2, &G2_IDENTITY) {
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
        }
    }

    Ok(())
}

// ==================== C FFI Functions ====================

/// BN254 batch pairing with validation, over little-endian `u64` limbs:
/// `result = e(P₁, Q₁) · e(P₂, Q₂) · ... · e(Pₙ, Qₙ)`.
///
/// # Safety
/// - `g1_ptr` must point to `num_points` `[u64; 8]` affine G1 points (x ‖ y)
/// - `g2_ptr` must point to `num_points` `[u64; 16]` affine G2 points (x_r ‖ x_i ‖ y_r ‖ y_i)
/// - `result_ptr` must point to a writable `[u64; 48]` array (Fq12)
///
/// # Returns
/// - [PAIRING_CHECK_SUCCESS] = success, the product is written to `result_ptr`
/// - [PAIRING_CHECK_ERR_G1_NOT_CANONICAL] .. [PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP] = an invalid
///   input point; `result_ptr` is left untouched
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_pairing_batch_safe_bn254_c")]
pub unsafe extern "C" fn pairing_batch_safe_bn254_c(
    g1_ptr: *const u64,
    g2_ptr: *const u64,
    num_points: usize,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let g1_points = slice_from_ffi(g1_ptr as *const [u64; 8], num_points);
    let g2_points = slice_from_ffi(g2_ptr as *const [u64; 16], num_points);

    match pairing_batch_safe_bn254(
        g1_points,
        g2_points,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(pairing) => {
            *(result_ptr as *mut [u64; 48]) = pairing;
            PAIRING_CHECK_SUCCESS
        }
        Err(code) => code,
    }
}

/// BN254 pairing check with big-endian byte format
///
/// # Safety
/// - `pairs` must point to an array of `num_pairs * 192` bytes
///   Each pair is: 64 bytes G1 point + 128 bytes G2 point
///
/// # Returns
/// - [PAIRING_CHECK_SUCCESS] = pairing check passed
/// - [PAIRING_CHECK_FAILED] = pairing check failed (result not 1)
/// - [PAIRING_CHECK_ERR_G1_NOT_CANONICAL] = error (at least one G1 point coordinate not in field)
/// - [PAIRING_CHECK_ERR_G1_NOT_ON_CURVE] = error (at least one G1 point not on curve)
/// - [PAIRING_CHECK_ERR_G2_NOT_CANONICAL] = error (at least one G2 point coordinate not in field)
/// - [PAIRING_CHECK_ERR_G2_NOT_ON_CURVE] = error (at least one G2 point not on curve)
/// - [PAIRING_CHECK_ERR_G2_NOT_IN_SUBGROUP] = error (at least one G2 point not in subgroup)
#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn pairing_check_safe_bn254_c(
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
    match pairing_check_safe_bn254(
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

#[cfg(all(test, not(feature = "hints")))]
mod tests {
    use super::*;
    use crate::zisklib::{
        frobenius1_fp12_bn254, frobenius2_fp12_bn254, frobenius3_fp12_bn254, mul_fp12_bn254,
    };
    use ark_bn254::{Fr, G1Projective, G2Projective};
    use ark_ec::{CurveGroup, PrimeGroup};
    use ark_ff::PrimeField;

    fn g1(k: u64, neg: bool) -> [u64; 8] {
        let p = (G1Projective::generator() * Fr::from(k)).into_affine();
        let p = if neg { -p } else { p };
        let mut o = [0u64; 8];
        o[..4].copy_from_slice(&p.x.into_bigint().0);
        o[4..].copy_from_slice(&p.y.into_bigint().0);
        o
    }

    fn g2(k: u64) -> [u64; 16] {
        let p = (G2Projective::generator() * Fr::from(k)).into_affine();
        let mut o = [0u64; 16];
        for (i, c) in [p.x.c0, p.x.c1, p.y.c0, p.y.c1].iter().enumerate() {
            o[4 * i..4 * i + 4].copy_from_slice(&c.into_bigint().0);
        }
        o
    }

    /// e(7G₁, 11G₂)·e(-77G₁, G₂) = 1: the host witness must pass the guest's check without the
    /// final exponentiation, and a product that is not 1 must get no witness
    #[test]
    fn miller_loop_witness_is_verified_without_the_final_exponentiation() {
        let (p, q) = ([g1(7, false), g1(77, true)], [g2(11), g2(1)]);
        let (c_inv, s) = fcall_bn254_final_exp_witness(&p, &q);
        assert!(!is_zero(&c_inv));
        let c = inv_fp12_bn254(&c_inv);
        let f = miller_loop_batch_with_witness_bn254(&p, &q, Some((&c_inv, &c)));
        let mut s12 = [0u64; 48];
        s12[..24].copy_from_slice(&s);
        let mut lhs = mul_fp12_bn254(&f, &s12);
        for t in [frobenius1_fp12_bn254(&c_inv), frobenius3_fp12_bn254(&c_inv)] {
            lhs = mul_fp12_bn254(&lhs, &t);
        }
        assert_eq!(lhs, frobenius2_fp12_bn254(&c_inv));
        assert!(is_zero(&fcall_bn254_final_exp_witness(&[p[0], p[0]], &q).0));
    }
}
