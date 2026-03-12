use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::path::{Path, PathBuf};
use zisk_common::ElfBinaryLike;

use crate::{
    gen_elf_hash, get_elf_bin_file_path_with_hash, get_elf_bin_verkey_file_path_with_hash,
    get_elf_data_hash, get_elf_vk, get_output_path, get_rom_info,
};

pub fn rom_merkle_setup<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &impl ElfBinaryLike,
    output_dir: &Option<PathBuf>,
) -> Result<(PathBuf, Vec<u8>), anyhow::Error> {
    let output_path = get_output_path(output_dir)?;

    let elf_hash = get_elf_data_hash(elf)?;

    let rom_info = get_rom_info(&pctx.global_info.get_proving_key_path())?;

    let elf_bin_path = get_elf_bin_file_path_with_hash(
        &elf_hash,
        &output_path,
        rom_info.blowup_factor,
        rom_info.merkle_tree_arity,
    )?;

    let elf_verkey_bin_path = get_elf_bin_verkey_file_path_with_hash(
        &elf_hash,
        &output_path,
        rom_info.blowup_factor,
        rom_info.merkle_tree_arity,
    )?;

    if elf_bin_path.exists() && elf_verkey_bin_path.exists() {
        let verkey = get_elf_vk(elf_verkey_bin_path.as_path())?
            .ok_or_else(|| anyhow::anyhow!("Failed to read existing verkey file"))?;

        return Ok((elf_bin_path, verkey));
    }

    let root = gen_elf_hash::<F>(
        pctx,
        elf.elf(),
        elf_bin_path.as_path(),
        rom_info.blowup_factor,
        rom_info.merkle_tree_arity,
    )?;

    tracing::info!("Root hash: {:?}", root);

    let verkey: Vec<u8> = root.iter().flat_map(|x| x.as_canonical_u64().to_le_bytes()).collect();

    std::fs::write(&elf_verkey_bin_path, &verkey)?;

    Ok((elf_bin_path, verkey))
}

pub fn rom_merkle_setup_verkey(
    elf: &impl ElfBinaryLike,
    output_dir: &Option<PathBuf>,
    proving_key: &Path,
) -> Result<Vec<u8>, anyhow::Error> {
    let output_path = get_output_path(output_dir)?;

    let elf_hash = get_elf_data_hash(elf)?;

    let rom_info = get_rom_info(proving_key)?;

    let elf_bin_path = get_elf_bin_file_path_with_hash(
        &elf_hash,
        &output_path,
        rom_info.blowup_factor,
        rom_info.merkle_tree_arity,
    )?;

    let elf_verkey_bin_path = get_elf_bin_verkey_file_path_with_hash(
        &elf_hash,
        &output_path,
        rom_info.blowup_factor,
        rom_info.merkle_tree_arity,
    )?;

    if elf_bin_path.exists() && elf_verkey_bin_path.exists() {
        let verkey = get_elf_vk(elf_verkey_bin_path.as_path())?
            .ok_or_else(|| anyhow::anyhow!("Failed to read existing verkey file"))?;

        Ok(verkey)
    } else {
        Err(anyhow::anyhow!("ROM merkle setup has not been performed yet"))
    }
}
