use fields::PrimeField64;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Result};
use zisk_common::ProgramVK;

use crate::{GuestProgram, ProgramId};
use proofman_common::{
    initialize_logger, json_to_debug_instances_map, DebugInfo, ProofCtx, ProofmanResult,
    VerboseMode,
};
use rom_setup::{get_elf_data_hash, get_rom_path, rom_merkle_setup, HashMode};

fn hash_mode_from_pctx<F: PrimeField64>(pctx: &ProofCtx<F>) -> Result<HashMode> {
    HashMode::from_str(&pctx.global_info.hash).with_context(|| {
        format!(
            "proving key global_info.hash {:?} is not a recognized HashMode",
            pctx.global_info.hash
        )
    })
}

pub fn ensure_program_vk<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &GuestProgram,
) -> Result<ProgramVK> {
    let hash_mode = hash_mode_from_pctx(pctx)?;
    rom_merkle_setup(pctx, elf.elf(), &None, false, hash_mode)
}

pub fn get_rom_bin_path<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    program_id: &ProgramId,
) -> Result<PathBuf> {
    let hash_mode = hash_mode_from_pctx(pctx)?;
    let rom_bin_path = get_rom_path(pctx, program_id.get_hash(), &None, hash_mode)?;
    Ok(rom_bin_path)
}

pub fn get_asm_paths(elf: &GuestProgram, with_hints: bool) -> Result<(String, String)> {
    let name = elf.name();
    let hash = get_elf_data_hash(elf.elf());
    let prefix = if name != hash { format!("{name}-{hash}") } else { hash };
    let base = if with_hints { format!("{prefix}-hints") } else { prefix };

    Ok((format!("{base}-mt.bin"), format!("{base}-rh.bin")))
}

pub fn check_paths_exist(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {:?}", path));
    }
    Ok(())
}

pub fn create_debug_info(
    debug_info: Option<Option<String>>,
    proving_key: PathBuf,
) -> ProofmanResult<DebugInfo> {
    match &debug_info {
        None => Ok(DebugInfo::default()),
        Some(None) => Ok(DebugInfo::new_debug()),
        Some(Some(debug_value)) => json_to_debug_instances_map(proving_key, debug_value.clone()),
    }
}

pub fn setup_logger(verbose: VerboseMode) {
    initialize_logger(verbose, None);
}
