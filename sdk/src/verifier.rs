use crate::{
    get_default_proving_key, get_default_proving_key_snark, ZiskProgramVK, ZiskProof, ZiskPublics,
};
use anyhow::{anyhow, Ok, Result};
use proofman::{get_vadcop_final_proof_vkey, verify_snark_proof, SnarkProof, SnarkProtocol};
use proofman_util::VadcopFinalProof;
use rom_setup::rom_merkle_setup_verkey;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use zisk_common::ElfBinaryLike;
use zisk_verifier::verify_vadcop_final_proof;

pub fn verify_zisk_snark_proof(
    proof: &ZiskProof,
    publics: &ZiskPublics,
    program_vk: &ZiskProgramVK,
) -> Result<()> {
    let proving_key = get_default_proving_key();
    let proving_key_snark = get_default_proving_key_snark();

    verify_zisk_snark_proof_with_proving_key(
        proof,
        publics,
        program_vk,
        proving_key,
        proving_key_snark,
    )
}

pub fn verify_zisk_proof(
    zisk_proof: &ZiskProof,
    publics: &ZiskPublics,
    program_vk: &ZiskProgramVK,
) -> Result<()> {
    let proving_key = get_default_proving_key();
    verify_zisk_proof_with_proving_key(zisk_proof, publics, program_vk, proving_key)
}

pub fn get_program_vk(elf: &impl ElfBinaryLike) -> Result<ZiskProgramVK> {
    let proving_key_path = get_default_proving_key();
    get_program_vk_with_proving_key(elf, proving_key_path)
}

pub fn verify_zisk_snark_proof_with_proving_key(
    proof: &ZiskProof,
    publics: &ZiskPublics,
    program_vk: &ZiskProgramVK,
    proving_key: PathBuf,
    proving_key_snark: PathBuf,
) -> Result<()> {
    match &proof {
        ZiskProof::Plonk(proof_bytes) | ZiskProof::Fflonk(proof_bytes) => {
            let protocol_id = if let ZiskProof::Plonk(_) = &proof {
                SnarkProtocol::Plonk.protocol_id()
            } else {
                SnarkProtocol::Fflonk.protocol_id()
            };

            if !proving_key_snark.exists() {
                return Err(anyhow!(
                    "Proving key snark path does not exist: {}",
                    proving_key_snark.display()
                ));
            }

            let verkey = get_vadcop_final_proof_vkey(&proving_key, false)?;

            let pubs = publics.bytes_solidity(program_vk, &verkey);
            let hash = Sha256::digest(&pubs).to_vec();

            let snark_proof = SnarkProof {
                proof_bytes: proof_bytes.clone(),
                public_bytes: pubs,
                public_snark_bytes: hash,
                protocol_id,
            };

            let verkey_path = PathBuf::from(format!(
                "{}/{}/{}.verkey.json",
                proving_key_snark.display(),
                "final",
                "final"
            ));
            Ok(verify_snark_proof(&snark_proof, &verkey_path)?)
        }
        _ => Err(anyhow!("Not a snark proof.")),
    }
}

pub fn verify_zisk_proof_with_proving_key(
    zisk_proof: &ZiskProof,
    publics: &ZiskPublics,
    program_vk: &ZiskProgramVK,
    proving_key: PathBuf,
) -> Result<()> {
    match &zisk_proof {
        ZiskProof::VadcopFinal(proof_bytes) | ZiskProof::VadcopFinalCompressed(proof_bytes) => {
            let compressed = matches!(zisk_proof, ZiskProof::VadcopFinalCompressed(_));
            let mut pubs = program_vk.vk.clone();
            pubs.extend(publics.public_bytes());
            let vadcop_final_proof = VadcopFinalProof::new(proof_bytes.clone(), pubs, compressed);

            let vk = get_vadcop_final_proof_vkey(&proving_key, compressed)?;
            verify_vadcop_final_proof(&vadcop_final_proof, &vk)
        }
        _ => Err(anyhow!("Not a Vadcop final proof.")),
    }
}

pub fn get_program_vk_with_proving_key(
    elf: &impl ElfBinaryLike,
    proving_key_path: PathBuf,
) -> Result<ZiskProgramVK> {
    let vk = rom_merkle_setup_verkey(elf, &None, &proving_key_path)?;
    Ok(ZiskProgramVK { vk })
}
