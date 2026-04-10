use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;

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

pub fn resolve_elf_path(elf: &Option<PathBuf>) -> Result<&PathBuf> {
    elf.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "No ELF available. Pass --elf or run from a Rust project with a built guest at target/riscv64ima-zisk-zkvm-elf/<binary-name>."
        )
    })
}

pub fn detect_current_project_elf() -> Result<Option<PathBuf>> {
    let current_dir = env::current_dir()?;
    let cargo_toml = current_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&cargo_toml)?;
    let binary_name = parse_package_name_from_cargo_toml(&content);

    let Some(binary_name) = binary_name else {
        return Ok(None);
    };

    let candidate = current_dir.join("target").join("riscv64ima-zisk-zkvm-elf");

    let release_candidate = candidate.join("release").join(&binary_name);
    if release_candidate.exists() {
        return Ok(Some(release_candidate));
    }

    let debug_candidate = candidate.join("debug").join(&binary_name);
    if debug_candidate.exists() {
        Ok(Some(debug_candidate))
    } else {
        Ok(None)
    }
}

fn parse_package_name_from_cargo_toml(content: &str) -> Option<String> {
    let mut in_package = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line == "[package]" {
            in_package = true;
            continue;
        }

        if line.starts_with('[') {
            in_package = false;
            continue;
        }

        if in_package && line.starts_with("name") {
            return parse_toml_string_value(line);
        }
    }

    None
}

fn parse_toml_string_value(line: &str) -> Option<String> {
    let (_, value) = line.split_once('=')?;
    let value = value.trim();
    if !(value.starts_with('"') && value.ends_with('"')) {
        return None;
    }
    Some(value.trim_matches('"').to_string())
}
