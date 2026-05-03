use fields::PrimeField64;
use std::path::PathBuf;

use anyhow::Result;
use zisk_common::{ProgramVK, ZiskPaths};

use crate::{GuestProgram, ProgramId};
use proofman_common::{
    initialize_logger, json_to_debug_instances_map, DebugInfo, ProofCtx, ProofmanResult,
    VerboseMode,
};
use rom_setup::{get_elf_data_hash, get_rom_path, rom_merkle_setup};

/// Gets the proving key file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key(proving_key: Option<&PathBuf>) -> PathBuf {
    proving_key.cloned().unwrap_or_else(|| ZiskPaths::global().proving_key.clone())
}

/// Gets the proving key file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key_snark(proving_key_snark: Option<&PathBuf>) -> PathBuf {
    proving_key_snark.cloned().unwrap_or_else(|| ZiskPaths::global().proving_key_snark.clone())
}

pub fn ensure_program_vk<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &GuestProgram,
) -> Result<ProgramVK> {
    rom_merkle_setup(pctx, elf.elf(), &None)
}

pub fn get_rom_bin_path<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    program_id: &ProgramId,
) -> Result<PathBuf> {
    let rom_bin_path = get_rom_path(pctx, program_id.get_hash(), &None)?;
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
