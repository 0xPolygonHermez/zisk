use std::env;
use std::path::PathBuf;

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

/// Gets the proving key file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key(proving_key: Option<&PathBuf>) -> PathBuf {
    proving_key.cloned().unwrap_or_else(get_default_proving_key)
}

/// Gets the proving key snark file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key_snark(proving_key_snark: Option<&PathBuf>) -> PathBuf {
    proving_key_snark.cloned().unwrap_or_else(get_default_proving_key_snark)
}

/// Gets the zisk folder.
/// Uses the default one if not specified by user.
pub fn get_zisk_path(zisk_path: Option<&PathBuf>) -> PathBuf {
    zisk_path.cloned().unwrap_or_else(get_default_zisk_path)
}
