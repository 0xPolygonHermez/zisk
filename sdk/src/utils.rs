use fields::Goldilocks;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;

use proofman_common::{json_to_debug_instances_map, DebugInfo, ProofmanResult};
use rom_setup::{get_elf_data_hash, rom_merkle_setup};
use zisk_common::ElfBinaryLike;

/// Gets the user's home directory as specified by the HOME environment variable.
pub fn get_home_dir() -> String {
    env::var("HOME").expect("get_home_dir() failed to get HOME environment variable")
}

/// Gets the default proving key file location in the home installation directory.
pub fn get_default_proving_key() -> PathBuf {
    let proving_key = format!("{}/.zisk/provingKey", get_home_dir());
    PathBuf::from(proving_key)
}

/// Gets the default proving key file location in the home installation directory.
pub fn get_default_proving_key_snark() -> PathBuf {
    let proving_key_snark = format!("{}/.zisk/provingKeySnark", get_home_dir());
    PathBuf::from(proving_key_snark)
}

/// Gets the default zisk folder location in the home installation directory.
pub fn get_home_zisk_path() -> PathBuf {
    let zisk_path = format!("{}/.zisk", get_home_dir());
    PathBuf::from(zisk_path)
}

/// Gets the default stark info JSON file location in the home installation directory.
pub fn get_default_stark_info() -> String {
    let stark_info = format!(
        "{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json",
        get_home_dir()
    );
    stark_info
}

/// Gets the default verifier binary file location in the home installation directory.
pub fn get_default_verifier_bin() -> String {
    let verifier_bin =
        format!("{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.verifier.bin", get_home_dir());
    verifier_bin
}

/// Gets the default verification key JSON file location in the home installation directory.
pub fn get_default_verkey() -> String {
    let verkey =
        format!("{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.verkey.bin", get_home_dir());
    verkey
}

/// If the target_os is macOS returns an error indicating that the command is not supported.
pub fn cli_fail_if_macos() -> anyhow::Result<()> {
    if cfg!(target_os = "macos") {
        Err(anyhow::anyhow!("Command is not supported on macOS"))
    } else {
        Ok(())
    }
}

/// Gets the proving key file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key(proving_key: Option<&PathBuf>) -> PathBuf {
    proving_key.cloned().unwrap_or_else(get_default_proving_key)
}

/// Gets the proving key file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key_snark(proving_key_snark: Option<&PathBuf>) -> PathBuf {
    proving_key_snark.cloned().unwrap_or_else(get_default_proving_key_snark)
}

pub fn ensure_custom_commits(
    proving_key: &Path,
    elf: &impl ElfBinaryLike,
) -> Result<(PathBuf, Vec<u8>)> {
    rom_merkle_setup::<Goldilocks>(elf, &None, proving_key)
}

pub fn get_custom_commits_map(
    proving_key: &Path,
    elf: &impl ElfBinaryLike,
) -> Result<HashMap<String, PathBuf>> {
    let (rom_bin_path, _) = ensure_custom_commits(proving_key, elf)?;
    Ok(HashMap::from([("rom".to_string(), rom_bin_path)]))
}

pub fn get_asm_paths(elf: &impl ElfBinaryLike) -> Result<(String, String)> {
    let stem = elf.name();
    let hash =
        get_elf_data_hash(elf).map_err(|e| anyhow::anyhow!("Error computing ELF hash: {}", e))?;

    Ok((format!("{stem}-{hash}-mt.bin"), format!("{stem}-{hash}-rh.bin")))
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
