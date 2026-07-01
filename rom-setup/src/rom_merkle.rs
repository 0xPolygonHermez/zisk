use anyhow::Result;
use fields::{Goldilocks, PrimeField64};
use proofman_common::{custom_commit_file_size_bytes, ProofCtx};
use std::path::{Path, PathBuf};
use zisk_common::ProgramVK;
use zisk_pil::RomRomTrace;

use crate::{
    gen_elf_hash, get_elf_bin_file_path_with_hash, get_elf_bin_verkey_file_path_with_hash,
    get_elf_data_hash, get_elf_vk, get_output_path, HashMode,
};

fn validate_custom_commit_file_size(elf_bin_path: &Path, hash_mode: HashMode) -> Result<()> {
    let n = RomRomTrace::<Goldilocks>::NUM_ROWS as u64;
    let n_cols = RomRomTrace::<Goldilocks>::ROW_SIZE as u64;
    let n_extended = hash_mode.blowup_factor() * n;
    let expected_size =
        custom_commit_file_size_bytes(n, n_extended, n_cols, hash_mode.merkle_tree_arity());

    let actual_size = std::fs::metadata(elf_bin_path)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to read cached ROM custom commit '{}': {e}",
                elf_bin_path.display()
            )
        })?
        .len();

    if actual_size != expected_size {
        return Err(anyhow::anyhow!(
            "Cached ROM custom commit '{}' has the wrong size (expected {expected_size} bytes, found {actual_size} bytes). \
            It was most likely generated with different setup parameters (blowup factor {}, merkle tree arity {}, hash mode {}) \
            or is stale/corrupted. Delete it (or re-run setup with force) so it is regenerated.",
            elf_bin_path.display(),
            hash_mode.blowup_factor(),
            hash_mode.merkle_tree_arity(),
            hash_mode.file_tag(),
        ));
    }

    Ok(())
}

pub fn get_rom_path<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf_hash: &str,
    output_dir: &Option<PathBuf>,
    hash_mode: HashMode,
) -> Result<PathBuf> {
    let output_path = get_output_path(output_dir)?;

    let elf_bin_path =
        get_elf_bin_file_path_with_hash(elf_hash, &output_path, pctx.gpu, hash_mode)?;

    let elf_verkey_bin_path =
        get_elf_bin_verkey_file_path_with_hash(elf_hash, &output_path, hash_mode)?;

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
    force: bool,
    hash_mode: HashMode,
) -> Result<ProgramVK, anyhow::Error> {
    let output_path = get_output_path(output_dir)?;

    let elf_hash = get_elf_data_hash(elf);

    let elf_bin_path =
        get_elf_bin_file_path_with_hash(&elf_hash, &output_path, pctx.gpu, hash_mode)?;

    let elf_verkey_bin_path =
        get_elf_bin_verkey_file_path_with_hash(&elf_hash, &output_path, hash_mode)?;

    if !force && elf_bin_path.exists() && elf_verkey_bin_path.exists() {
        validate_custom_commit_file_size(&elf_bin_path, hash_mode)?;

        let vk = get_elf_vk(elf_verkey_bin_path.as_path())?
            .ok_or_else(|| anyhow::anyhow!("Failed to read existing verkey file"))?;
        return Ok(ProgramVK { vk, hash_mode });
    }

    let root = gen_elf_hash::<F>(pctx, elf, elf_bin_path.as_path(), hash_mode)?;

    tracing::info!("Root hash: {:?}", root);

    let vk: Vec<u64> = root.iter().map(|x| x.as_canonical_u64()).collect();

    let vk_bytes: Vec<u8> = vk.iter().flat_map(|w| w.to_le_bytes()).collect();
    std::fs::write(&elf_verkey_bin_path, &vk_bytes)?;

    Ok(ProgramVK { vk, hash_mode })
}

pub fn rom_merkle_setup_verkey(
    elf: &[u8],
    output_dir: &Option<PathBuf>,
    hash_mode: HashMode,
) -> Result<ProgramVK, anyhow::Error> {
    let output_path = get_output_path(output_dir)?;

    let elf_hash = get_elf_data_hash(elf);

    let elf_verkey_bin_path =
        get_elf_bin_verkey_file_path_with_hash(&elf_hash, &output_path, hash_mode)?;

    if elf_verkey_bin_path.exists() {
        let vk = get_elf_vk(elf_verkey_bin_path.as_path())?
            .ok_or_else(|| anyhow::anyhow!("Failed to read existing verkey file"))?;

        Ok(ProgramVK { vk, hash_mode })
    } else {
        Err(anyhow::anyhow!("ROM merkle setup has not been performed yet"))
    }
}
