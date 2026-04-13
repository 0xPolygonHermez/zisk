use crate::{ROM_BLOWUP_FACTOR, ROM_MERKLE_TREE_ARITY};
use anyhow::Result;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::path::PathBuf;

use crate::{
    gen_elf_hash, get_elf_bin_file_path_with_hash, get_elf_bin_verkey_file_path_with_hash,
    get_elf_data_hash, get_elf_vk, get_output_path,
};

pub fn get_rom_path<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf_hash: &str,
    output_dir: &Option<PathBuf>,
) -> Result<PathBuf> {
    let output_path = get_output_path(output_dir)?;

    let elf_bin_path = get_elf_bin_file_path_with_hash(elf_hash, &output_path, pctx.gpu)?;

    let elf_verkey_bin_path = get_elf_bin_verkey_file_path_with_hash(elf_hash, &output_path)?;

    if !elf_bin_path.exists() || !elf_verkey_bin_path.exists() {
        return Err(anyhow::anyhow!(
            "ROM files not found for ELF hash {}. Expected paths: {:?} and {:?}",
            elf_hash,
            elf_bin_path,
            elf_verkey_bin_path
        ));
    }

    Ok(elf_bin_path)
}
pub fn rom_merkle_setup<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &[u8],
    output_dir: &Option<PathBuf>,
) -> Result<PathBuf, anyhow::Error> {
    let output_path = get_output_path(output_dir)?;

    let elf_hash = get_elf_data_hash(elf)?;

    let elf_bin_path = get_elf_bin_file_path_with_hash(&elf_hash, &output_path, pctx.gpu)?;

    let elf_verkey_bin_path = get_elf_bin_verkey_file_path_with_hash(&elf_hash, &output_path)?;

    if elf_bin_path.exists() && elf_verkey_bin_path.exists() {
        return Ok(elf_bin_path);
    }

    let root = gen_elf_hash::<F>(
        pctx,
        elf,
        elf_bin_path.as_path(),
        ROM_BLOWUP_FACTOR,
        ROM_MERKLE_TREE_ARITY,
    )?;

    tracing::info!("Root hash: {:?}", root);

    let verkey: Vec<u8> = root.iter().flat_map(|x| x.as_canonical_u64().to_le_bytes()).collect();

    std::fs::write(&elf_verkey_bin_path, &verkey)?;

    Ok(elf_bin_path)
}

pub fn rom_merkle_setup_verkey(
    elf: &[u8],
    output_dir: &Option<PathBuf>,
) -> Result<Vec<u8>, anyhow::Error> {
    let output_path = get_output_path(output_dir)?;

    let elf_hash = get_elf_data_hash(elf)?;

    let elf_verkey_bin_path = get_elf_bin_verkey_file_path_with_hash(&elf_hash, &output_path)?;

    if elf_verkey_bin_path.exists() {
        let verkey = get_elf_vk(elf_verkey_bin_path.as_path())?
            .ok_or_else(|| anyhow::anyhow!("Failed to read existing verkey file"))?;

        Ok(verkey)
    } else {
        Err(anyhow::anyhow!("ROM merkle setup has not been performed yet"))
    }
}
