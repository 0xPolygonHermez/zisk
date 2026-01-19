use anyhow::{anyhow, Ok, Result};
use proofman_verifier::{verify_vadcop_final, verify_vadcop_final_compressed};

pub fn verify_zisk_proof(zisk_proof: &[u8], vk: &[u8]) -> Result<()> {
    if !verify_vadcop_final(zisk_proof, vk) {
        Err(anyhow!("Zisk Proof was not verified"))
    } else {
        Ok(())
    }
}

pub fn verify_zisk_proof_compressed(zisk_proof: &[u8], vk: &[u8]) -> Result<()> {
    if !verify_vadcop_final_compressed(zisk_proof, vk) {
        Err(anyhow!("Zisk Proof was not verified"))
    } else {
        Ok(())
    }
}
