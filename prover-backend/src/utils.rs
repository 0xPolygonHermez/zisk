use fields::PrimeField64;
use std::path::PathBuf;

use anyhow::Result;
use zisk_common::ProgramVK;

use crate::{GuestProgram, ProgramId};
use proofman_common::{
    initialize_logger, json_to_debug_instances_map, DebugInfo, ProofCtx, ProofmanResult,
    VerboseMode,
};
use rom_setup::{get_elf_data_hash, get_rom_path, rom_merkle_setup};

pub fn ensure_program_vk<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &GuestProgram,
) -> Result<ProgramVK> {
    rom_merkle_setup(pctx, elf.elf(), &None, false)
}

pub fn get_rom_bin_path<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    program_id: &ProgramId,
) -> Result<PathBuf> {
    let rom_bin_path = get_rom_path(pctx, program_id.get_hash(), &None)?;
    Ok(rom_bin_path)
}

pub fn get_asm_paths(elf: &GuestProgram, with_hints: bool) -> Result<(String, String)> {
    // Content-addressed by the ELF hash only — the same ELF maps to the same artifacts
    // regardless of the program name, so a given hash is generated once.
    let hash = get_elf_data_hash(elf.elf());
    let base = if with_hints { format!("{hash}-hints") } else { hash };

    Ok((format!("{base}-mt.bin"), format!("{base}-rh.bin")))
}

pub fn check_paths_exist(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {:?}", path));
    }
    Ok(())
}

/// Maps the CLI `--debug` flag (CLI-native `Option<Option<String>>`) to an in-memory
/// `Option<DebugInfo>`:
/// - `None` (no `--debug`)        → `None`             — let the proofman library
///   pick its own default (e.g. `verify_proof_constraints_from_lib` treats `None`
///   as "just verify").
/// - `Some(None)` (`--debug`)     → `Some(new_debug)`  — full debug-everything mode.
/// - `Some(Some(path))`           → `Some(from_json)`  — load from a JSON file.
pub fn create_debug_info(
    debug_info: Option<Option<String>>,
    proving_key: PathBuf,
) -> ProofmanResult<Option<DebugInfo>> {
    match &debug_info {
        None => Ok(None),
        Some(None) => Ok(Some(DebugInfo::new_debug())),
        Some(Some(debug_value)) => {
            json_to_debug_instances_map(proving_key, debug_value.clone()).map(Some)
        }
    }
}

pub fn setup_logger(verbose: VerboseMode) {
    initialize_logger(verbose, None);
}
