use proofman_fields::PrimeField64;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use zisk_common::ProgramVK;

use crate::{GuestProgram, ProgramId};
use proofman_common::{
    initialize_logger, json_to_debug_instances_map, DebugInfo, ProofCtx, ProofmanResult,
    VerboseMode,
};
use zisk_rom_setup::{
    get_elf_bin_verkey_file_path_with_hash, get_elf_data_hash, get_output_path, get_rom_path,
    rom_merkle_setup, HashMode,
};

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

/// Whether the cache holds the ROM setup of program `hash_id` for a proving key of hash family
/// `hash` (its global info's `hash`): the verkey file a setup writes and later reuses, named by
/// the program, the PIL and the hash mode. Another PIL or hash mode, or an emptied cache, misses.
pub fn has_cached_program_setup(hash_id: &str, hash: &str) -> Result<bool> {
    let hash_mode = HashMode::from_str(hash)
        .with_context(|| format!("proving key hash {hash:?} is not a recognized HashMode"))?;
    let output_path = get_output_path(&None)?;
    Ok(get_elf_bin_verkey_file_path_with_hash(hash_id, &output_path, hash_mode)?.exists())
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
/// Named by `zisk_rom_setup`, which generates these files, so the two cannot
/// disagree: the name is the ELF hash and the hints mode.
pub fn get_asm_paths(elf: &GuestProgram, with_hints: bool) -> Result<(String, String)> {
    let hash = get_elf_data_hash(elf.elf());
    let [mt, rh, ..] =
        zisk_rom_setup::get_assembly_file_paths_from_id(&hash, Path::new(""), with_hints);
    let file_name = |path: PathBuf| path.to_string_lossy().into_owned();

    Ok((file_name(mt), file_name(rh)))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A program's setup is found for the PIL and hash mode it was made for, and no other.
    #[test]
    fn a_cached_setup_is_found_for_its_hash_mode_only() {
        let hash_id = format!("test-cached-setup-{}", std::process::id());
        let output_path = get_output_path(&None).unwrap();
        let verkey =
            get_elf_bin_verkey_file_path_with_hash(&hash_id, &output_path, HashMode::Blake3)
                .unwrap();
        assert!(!has_cached_program_setup(&hash_id, "blake3").unwrap());

        std::fs::write(&verkey, [0u8; 32]).unwrap();
        let found = (
            has_cached_program_setup(&hash_id, "blake3").unwrap(),
            has_cached_program_setup(&hash_id, "Poseidon2").unwrap(),
        );
        std::fs::remove_file(&verkey).unwrap();

        assert_eq!(found, (true, false));
        assert!(has_cached_program_setup(&hash_id, "sha256").is_err());
    }
}
