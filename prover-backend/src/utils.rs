use proofman_fields::PrimeField64;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Result};
use zisk_common::ProgramVK;

use crate::{GuestProgram, ProgramId};
use proofman_common::{
    initialize_logger, json_to_debug_instances_map, DebugInfo, ProofCtx, ProofmanResult,
    VerboseMode,
};
use zisk_rom_setup::{get_elf_data_hash, get_rom_path, rom_merkle_setup, HashMode};

fn hash_mode_from_pctx<F: PrimeField64>(pctx: &ProofCtx<F>) -> Result<HashMode> {
    HashMode::from_str(&pctx.global_info.hash).with_context(|| {
        format!(
            "proving key global_info.hash {:?} is not a recognized HashMode",
            pctx.global_info.hash
        )
    })
}

/// Build the ROM Merkle setup for `elf` and return its program verification
/// key, using the hash mode declared in the proving key's global info.
pub fn ensure_program_vk<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &GuestProgram,
) -> Result<ProgramVK> {
    let hash_mode = hash_mode_from_pctx(pctx)?;
    rom_merkle_setup(pctx, elf.elf(), &None, false, hash_mode)
}

/// Resolve the on-disk path of the compiled ROM binary for `program_id`,
/// using the hash mode declared in the proving key's global info.
pub fn get_rom_bin_path<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    program_id: &ProgramId,
) -> Result<PathBuf> {
    let hash_mode = hash_mode_from_pctx(pctx)?;
    let rom_bin_path = get_rom_path(pctx, program_id.get_hash(), &None, hash_mode)?;
    Ok(rom_bin_path)
}

/// Return the `(minimal-trace, rom-histogram)` ASM binary filenames for `elf`.
///
/// Delegates the name to `zisk_rom_setup`, which is what generates these files.
/// Rebuilding the name here instead would be a second definition of it, free to
/// drift from the generator's — and it did: the protocol generation is part of
/// the name now, and a resolver that did not know that would look for artifacts
/// nobody writes.
pub fn get_asm_paths(elf: &GuestProgram, with_hints: bool) -> Result<(String, String)> {
    let hash = get_elf_data_hash(elf.elf());
    let base = zisk_rom_setup::asm_file_base(&hash, with_hints);

    Ok((format!("{base}-mt.bin"), format!("{base}-rh.bin")))
}

/// Return an error if `path` does not exist.
pub fn check_paths_exist(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {:?}", path));
    }
    Ok(())
}

/// Translate the CLI-style debug selector into a [`DebugInfo`]:
/// `None` → no debugging, `Some(None)` → debug all instances,
/// `Some(Some(spec))` → debug only the instances named in `spec`.
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

/// Initialize the global logger at the given verbosity (non-distributed).
pub fn setup_logger(verbose: VerboseMode) {
    initialize_logger(verbose, None);
}
