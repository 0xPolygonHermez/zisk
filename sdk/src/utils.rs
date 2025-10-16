use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::Result;

use proofman_common::{json_to_debug_instances_map, DebugInfo};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};

/// Gets the user's home directory as specified by the HOME environment variable.
pub fn get_home_dir() -> String {
    env::var("HOME").expect("get_home_dir() failed to get HOME environment variable")
}

/// Gets the default witness computation library file location in the home installation directory.
pub fn get_default_witness_computation_lib() -> PathBuf {
    let extension = if cfg!(target_os = "macos") { "dylib" } else { "so" };
    let witness_computation_lib =
        format!("{}/.zisk/bin/libzisk_witness.{}", get_home_dir(), extension);
    PathBuf::from(witness_computation_lib)
}

/// Gets the default proving key file location in the home installation directory.
pub fn get_default_proving_key() -> PathBuf {
    let proving_key = format!("{}/.zisk/provingKey", get_home_dir());
    PathBuf::from(proving_key)
}

/// Gets the default zisk folder location in the home installation directory.
pub fn get_home_zisk_path() -> PathBuf {
    let zisk_path = format!("{}/.zisk", get_home_dir());
    PathBuf::from(zisk_path)
}

/// Gets the default zisk folder location in the home installation directory.
pub fn get_default_zisk_path() -> PathBuf {
    let zisk_path = format!("{}/.zisk/zisk", get_home_dir());
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

/// If the feature "gpu" is enabled, returns an error indicating that the command is not supported.
pub fn cli_fail_if_gpu_mode() -> anyhow::Result<()> {
    if cfg!(feature = "gpu") {
        Err(anyhow::anyhow!("Command is not supported on GPU mode"))
    } else {
        Ok(())
    }
}

/// Gets the witness computation library file location.
/// Uses the default one if not specified by user.
pub fn get_witness_computation_lib(witness_lib: Option<&PathBuf>) -> PathBuf {
    witness_lib.cloned().unwrap_or_else(get_default_witness_computation_lib)
}

/// Gets the proving key file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key(proving_key: Option<&PathBuf>) -> PathBuf {
    proving_key.cloned().unwrap_or_else(get_default_proving_key)
}

/// Gets the zisk folder.
/// Uses the default one if not specified by user.
pub fn get_zisk_path(zisk_path: Option<&PathBuf>) -> PathBuf {
    zisk_path.cloned().unwrap_or_else(get_default_zisk_path)
}

pub fn ensure_custom_commits(proving_key: &Path, elf: &Path) -> Result<PathBuf> {
    // Ensure cache directory exists
    let default_cache_path = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|e| anyhow::anyhow!("Failed to read HOME environment variable: {e}"))?
        .join(DEFAULT_CACHE_PATH);

    if let Err(e) = fs::create_dir_all(&default_cache_path) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Failed to create cache directory: {e:?}");
        }
    }

    // Get the blowup factor as the custom commits filename is formed using it
    // {ELF_HASH}_{PILOUT_HASH}_{ROM_NUM_ROWS}_{BLOWUP_FACTOR}.bin
    let blowup_factor = get_rom_blowup_factor(proving_key);

    // Compute the path for the custom commits file
    let rom_bin_path = get_elf_bin_file_path(elf, &default_cache_path, blowup_factor)?;

    // Check if the custom commits file exists, if not generate it
    if !rom_bin_path.exists() {
        let _ = gen_elf_hash(elf, rom_bin_path.as_path(), blowup_factor, false)
            .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
    }

    Ok(rom_bin_path)
}

pub fn get_custom_commits_map(proving_key: &Path, elf: &Path) -> Result<HashMap<String, PathBuf>> {
    let rom_bin_path = ensure_custom_commits(proving_key, elf)?;
    Ok(HashMap::from([("rom".to_string(), rom_bin_path)]))
}

pub fn get_asm_paths(elf: &Path) -> Result<(String, String)> {
    let stem = elf.file_stem().unwrap().to_str().unwrap();
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

pub fn create_debug_info(debug_info: Option<Option<String>>, proving_key: PathBuf) -> DebugInfo {
    match &debug_info {
        None => DebugInfo::default(),
        Some(None) => DebugInfo::new_debug(),
        Some(Some(debug_value)) => json_to_debug_instances_map(proving_key, debug_value.clone()),
    }
}
