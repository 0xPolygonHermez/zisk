use crate::zisklib::{
    is_on_subgroup_bls12_381,
    lib::utils::{eq, is_one, lt},
};

use super::{
    constants::{G1_GENERATOR, G1_IDENTITY, G2_GENERATOR, G2_IDENTITY, R, TRUSTED_SETUP_TAU_G2},
    curve::{decompress_bls12_381, scalar_mul_bls12_381, sub_bls12_381, sub_complete_bls12_381},
    pairing::pairing_batch_bls12_381,
    twist::{
        decompress_twist_bls12_381, neg_twist_bls12_381, scalar_mul_twist_bls12_381,
        sub_complete_twist_bls12_381, sub_twist_bls12_381,
    },
};

/// Verify KZG proof using BLS12-381 implementation.
pub fn verify_kzg_proof(
    z_bytes: &[u8; 32],
    y_bytes: &[u8; 32],
    commitment_bytes: &[u8; 48],
    proof_bytes: &[u8; 48],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // Parse the commitment
    let commitment = match decompress_bls12_381(
        commitment_bytes,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(result) => result,
        Err(_) => return false,
    };
    if !eq(&commitment, &G1_IDENTITY)
        && !is_on_subgroup_bls12_381(
            &commitment,
            #[cfg(feature = "hints")]
            hints,
        )
    {
        return false;
    }

    // Parse the proof
    let proof = match decompress_bls12_381(
        proof_bytes,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok(result) => result,
        Err(_) => return false,
    };
    if !eq(&proof, &G1_IDENTITY)
        && !is_on_subgroup_bls12_381(
            &proof,
            #[cfg(feature = "hints")]
            hints,
        )
    {
        return false;
    }

    // Parse z and y as scalar field elements
    let z = match scalar_bytes_be_to_u64_le_canonical(z_bytes) {
        Some(s) => s,
        None => return false,
    };

    let y = match scalar_bytes_be_to_u64_le_canonical(y_bytes) {
        Some(s) => s,
        None => return false,
    };

    // The verification equation is:
    // e(C - [y]G₁, G₂) = e(π, [τ]₂ - [z]G₂)

    // Get the trusted setup G2 point [τ]₂
    let tau_g2 = TRUSTED_SETUP_TAU_G2;

    // Get generators
    let g1 = G1_GENERATOR;
    let g2 = G2_GENERATOR;

    // Compute c_minus_y = C - [y]G₁
    let y_g1 = scalar_mul_bls12_381(
        &g1,
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let c_minus_y = sub_complete_bls12_381(
        &commitment,
        &y_g1,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute t_minus_z = [τ]₂ - [z]G₂
    let z_g2 = scalar_mul_twist_bls12_381(
        &g2,
        &z,
        #[cfg(feature = "hints")]
        hints,
    );
    let t_minus_z = sub_complete_twist_bls12_381(
        &tau_g2,
        &z_g2,
        #[cfg(feature = "hints")]
        hints,
    );

    // LHS: e(C - [y]G₁, G₂) - G₂ is never infinity
    // RHS: e(π, [τ]₂ - [z]G₂)
    let c_minus_y_is_inf = eq(&c_minus_y, &G1_IDENTITY);
    let proof_is_inf = eq(&proof, &G1_IDENTITY);
    let t_minus_z_is_inf = eq(&t_minus_z, &G2_IDENTITY);

    // If c_minus_y = O: LHS = e(O, G₂) = 1
    //   => RHS must equal 1, i.e., e(π, [τ]₂ - [z]G₂) = 1
    //   => π = O or [τ]₂ - [z]G₂ = O
    if c_minus_y_is_inf {
        return proof_is_inf || t_minus_z_is_inf;
    }

    // If π = O or [τ]₂ - [z]G₂ = O: RHS = 1
    //   => LHS must equal 1, i.e., e(C - [y]G₁, G₂) = 1
    //   => C - [y]G₁ = O (but we already handled that above)
    //   => This means c_minus_y ≠ O but RHS = 1, so verification fails
    if proof_is_inf || t_minus_z_is_inf {
        return false;
    }

    // General case: no infinities, proceed with pairing check
    // The check is equivalent to:
    // e(C - [y]G₁, -G₂) · e(π, [τ]₂ - [z]G₂) = 1
    let neg_g2 = neg_twist_bls12_381(
        &g2,
        #[cfg(feature = "hints")]
        hints,
    );

    // Batch pairing check
    let g1_points = [c_minus_y, proof];
    let g2_points = [neg_g2, t_minus_z];

    // Check if the pairing result equals 1
    is_one(&pairing_batch_bls12_381(
        &g1_points,
        &g2_points,
        #[cfg(feature = "hints")]
        hints,
    ))
}

/// Verify KZG proof using BLS12-381 implementation.
///
/// # Arguments
/// * `z` - 32 bytes big-endian scalar (evaluation point)
/// * `y` - 32 bytes big-endian scalar (claimed evaluation)
/// * `commitment` - 48 bytes compressed G1 point (polynomial commitment)
/// * `proof` - 48 bytes compressed G1 point (KZG proof)
///
/// # Safety
/// All pointers must be valid and properly aligned.
///
/// # Returns
/// * 1 if the proof is valid
/// * 0 if the proof is invalid
/// * 2 if there was a parsing error (invalid input)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_verify_kzg_proof_c")]
pub unsafe extern "C" fn verify_kzg_proof_c(
    z: *const u8,
    y: *const u8,
    commitment: *const u8,
    proof: *const u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let z_bytes: &[u8; 32] = &*(z as *const [u8; 32]);
    let y_bytes: &[u8; 32] = &*(y as *const [u8; 32]);
    let commitment_bytes: &[u8; 48] = &*(commitment as *const [u8; 48]);
    let proof_bytes: &[u8; 48] = &*(proof as *const [u8; 48]);

    verify_kzg_proof(
        z_bytes,
        y_bytes,
        commitment_bytes,
        proof_bytes,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Convert 32-byte big-endian scalar to [u64; 4] little-endian, checking canonicity
/// Returns None if the scalar is not canonical (>= R)
fn scalar_bytes_be_to_u64_le_canonical(bytes: &[u8; 32]) -> Option<[u64; 4]> {
    // Convert big-endian bytes to little-endian u64 limbs
    let mut scalar = [0u64; 4];
    for i in 0..4 {
        for j in 0..8 {
            scalar[3 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // Check if scalar < R
    if !lt(&scalar, &R) {
        return None;
    }

    Some(scalar)
}
