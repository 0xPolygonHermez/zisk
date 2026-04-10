use std::env;
use std::path::PathBuf;

use anyhow::Result;

/// Gets the user's home directory from the HOME environment variable.
pub fn get_home_dir() -> Result<String> {
    env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable is not set"))
}

/// Gets the default proving key file location in the home installation directory.
pub fn get_default_proving_key() -> Result<PathBuf> {
    Ok(PathBuf::from(format!("{}/.zisk/provingKey", get_home_dir()?)))
}

/// Gets the default proving key snark file location in the home installation directory.
pub fn get_default_proving_key_snark() -> Result<PathBuf> {
    Ok(PathBuf::from(format!("{}/.zisk/provingKeySnark", get_home_dir()?)))
}

/// Gets the default zisk folder location in the home installation directory.
pub fn get_home_zisk_path() -> Result<PathBuf> {
    Ok(PathBuf::from(format!("{}/.zisk", get_home_dir()?)))
}

/// Gets the default stark info JSON file location in the home installation directory.
pub fn get_default_stark_info() -> Result<String> {
    Ok(format!(
        "{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json",
        get_home_dir()?
    ))
}

/// Gets the default verifier binary file location in the home installation directory.
pub fn get_default_verifier_bin() -> Result<String> {
    Ok(format!("{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.verifier.bin", get_home_dir()?))
}

/// Gets the default verification key JSON file location in the home installation directory.
pub fn get_default_verkey() -> Result<String> {
    Ok(format!("{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.verkey.bin", get_home_dir()?))
}

/// If the target_os is macOS returns an error indicating that the command is not supported.
pub fn cli_fail_if_macos() -> Result<()> {
    if cfg!(target_os = "macos") {
        Err(anyhow::anyhow!("Command is not supported on macOS"))
    } else {
        Ok(())
    }
}

/// Gets the proving key file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key(proving_key: Option<&PathBuf>) -> Result<PathBuf> {
    match proving_key {
        Some(p) => Ok(p.clone()),
        None => get_default_proving_key(),
    }
}

/// Gets the proving key snark file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key_snark(proving_key_snark: Option<&PathBuf>) -> Result<PathBuf> {
    match proving_key_snark {
        Some(p) => Ok(p.clone()),
        None => get_default_proving_key_snark(),
    }
}
