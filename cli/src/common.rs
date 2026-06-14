use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;
use zisk_build::ZISK_TARGET;

/// If the target_os is macOS returns an error indicating that the command is not supported.
pub fn cli_fail_if_macos() -> Result<()> {
    if cfg!(target_os = "macos") {
        Err(anyhow::anyhow!("Command is not supported on macOS"))
    } else {
        Ok(())
    }
}

pub fn resolve_elf_path(elf: &Option<PathBuf>) -> Result<&PathBuf> {
    elf.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "No ELF available. Pass --elf or run from a Rust project with a built guest at target/elf/riscv64ima-zisk-zkvm-elf/<binary-name>."
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

    let candidate = current_dir.join("target").join("elf").join(ZISK_TARGET);

    let release_candidate = candidate.join("release").join(&binary_name);
    if release_candidate.exists() {
        return Ok(Some(release_candidate));
    }

    let debug_candidate = candidate.join("debug").join(&binary_name);
    if debug_candidate.exists() {
        return Ok(Some(debug_candidate));
    }

    // Also probe the wasm guest artifact (built with `cargo-zisk build --machine wasm`).
    let wasm_dir =
        current_dir.join("target").join("elf").join(zisk_build::ZISK_WASM_TARGET);
    for profile in ["release", "debug"] {
        let wasm_candidate = wasm_dir.join(profile).join(format!("{binary_name}.wasm"));
        if wasm_candidate.exists() {
            return Ok(Some(wasm_candidate));
        }
    }

    Ok(None)
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
