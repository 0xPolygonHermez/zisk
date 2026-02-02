use crate::{handlers::validate_hint_length, hint_fields, zisklib};

use anyhow::Result;

/// Processes an `HINT_VERIFY_KZG_PROOF` hint.
#[inline]
pub fn verify_kzg_proof_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![Z: 32, Y: 32, COMMITMENT: 48, PROOF: 48];

    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 8) };

    validate_hint_length(bytes, EXPECTED_LEN, "HINT_VERIFY_KZG_PROOF")?;

    let z: &[u8; Z_SIZE] = bytes[Z_OFFSET..Z_OFFSET + Z_SIZE].try_into().unwrap();
    let y: &[u8; Y_SIZE] = bytes[Y_OFFSET..Y_OFFSET + Y_SIZE].try_into().unwrap();
    let commitment: &[u8; COMMITMENT_SIZE] =
        bytes[COMMITMENT_OFFSET..COMMITMENT_OFFSET + COMMITMENT_SIZE].try_into().unwrap();
    let proof: &[u8; PROOF_SIZE] =
        bytes[PROOF_OFFSET..PROOF_OFFSET + PROOF_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    unsafe {
        zisklib::verify_kzg_proof_c(
            z.as_ptr(),
            y.as_ptr(),
            commitment.as_ptr(),
            proof.as_ptr(),
            &mut hints,
        )
    };

    Ok(hints)
}
