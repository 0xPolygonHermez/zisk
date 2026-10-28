use crate::zisklib::{
    is_on_subgroup_bls12_381,
    lib::utils::{eq, is_one, lt},
};

use super::{
    constants::{G1_GENERATOR, G1_IDENTITY, G2_GENERATOR, R, TRUSTED_SETUP_TAU_G2},
    curve::{
        add_complete_bls12_381, decompress_bls12_381, scalar_mul_bls12_381, sub_complete_bls12_381,
    },
    pairing::pairing_batch_bls12_381,
    twist::neg_twist_bls12_381,
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
    let (commitment, commitment_is_inf) = match decompress_bls12_381(
        commitment_bytes,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok((result, is_infinity)) => (result, is_infinity),
        Err(_) => return false,
    };
    if !commitment_is_inf
        && !is_on_subgroup_bls12_381(
            &commitment,
            #[cfg(feature = "hints")]
            hints,
        )
    {
        return false;
    }

    // Parse the proof
    let (proof, proof_is_inf) = match decompress_bls12_381(
        proof_bytes,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Ok((result, is_infinity)) => (result, is_infinity),
        Err(_) => return false,
    };
    if !proof_is_inf
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

    // The verification equation e(C - [y]G₁, -G₂) · e(π, [τ]₂ - [z]G₂) = 1 is checked as
    // e(C - [y]G₁ + [z]π, -G₂) · e(π, [τ]₂) = 1, since e(π, [τ]₂ - [z]G₂) =
    // e(π, [τ]₂) · e([z]π, -G₂): a G1 scalar mul instead of a G2 one. This must emit the
    // same precompile and fcall sequence as kzg.zisk, which reads it back in hints mode.

    // Compute c_minus_y = C - [y]G₁
    let y_g1 = scalar_mul_bls12_381(
        &G1_GENERATOR,
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

    // π = O: the product is e(C - [y]G₁, -G₂), 1 iff C - [y]G₁ = O
    if eq(&proof, &G1_IDENTITY) {
        return eq(&c_minus_y, &G1_IDENTITY);
    }

    // P1 = (C - [y]G₁) + [z]π
    let z_proof = scalar_mul_bls12_381(
        &proof,
        &z,
        #[cfg(feature = "hints")]
        hints,
    );
    let p1 = add_complete_bls12_381(
        &c_minus_y,
        &z_proof,
        #[cfg(feature = "hints")]
        hints,
    );

    // P1 = O leaves e(π, [τ]₂), never 1 for π ≠ O
    if eq(&p1, &G1_IDENTITY) {
        return false;
    }

    let neg_g2 = neg_twist_bls12_381(
        &G2_GENERATOR,
        #[cfg(feature = "hints")]
        hints,
    );

    // Batch pairing check
    let g1_points = [p1, proof];
    let g2_points = [neg_g2, TRUSTED_SETUP_TAU_G2];

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
#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn verify_kzg_proof_c(
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
