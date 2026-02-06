use anyhow::{anyhow, Ok, Result};
use proofman_util::VadcopFinalProof;
use proofman_verifier::{verify_vadcop_final, verify_vadcop_final_compressed};

pub fn verify_vadcop_final_proof(
    zisk_proof: &VadcopFinalProof,
    vadcop_final_vk: &[u8],
) -> Result<()> {
    let is_valid = if zisk_proof.compressed {
        verify_vadcop_final_compressed(zisk_proof, vadcop_final_vk)
    } else {
        verify_vadcop_final(zisk_proof, vadcop_final_vk)
    };

    if !is_valid {
        Err(anyhow!("Zisk Proof was not verified"))
    } else {
        Ok(())
    }
}
