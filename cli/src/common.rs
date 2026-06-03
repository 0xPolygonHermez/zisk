use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;
use zisk_build::ZISK_TARGET;

/// Build the default proof output filename when the user passes no `--output`.
///
/// Format: `<timestamp>-<jobid if any>-proof[-plonk].bin`, where `<timestamp>`
/// is the current Unix time in seconds and the `-plonk` suffix is added only
/// for PLONK proofs.
pub(crate) fn default_proof_filename(job_id: Option<impl std::fmt::Display>) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let job_segment = job_id.map(|id| format!("{id}-")).unwrap_or_default();
    PathBuf::from(format!("{timestamp}-{job_segment}proof.bin"))
}

pub(crate) fn detect_current_project_elf() -> Result<Option<PathBuf>> {
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
        Ok(Some(debug_candidate))
    } else {
        Ok(None)
    }
}

/// Reject a `quic://` hints URI — the CLI has no event loop to host a live QUIC
/// stream, so it cannot serve QUIC hints to either the embedded or remote backend.
pub(crate) fn reject_quic_hints(hints: Option<&str>) -> Result<()> {
    if hints.is_some_and(|uri| uri.starts_with("quic://")) {
        anyhow::bail!("QUIC hints source is not supported in CLI mode.");
    }
    Ok(())
}

/// Resolve the guest ELF: explicit path, otherwise auto-detect from the current project.
pub(crate) fn resolve_elf(elf: Option<PathBuf>) -> Result<PathBuf> {
    match elf {
        Some(elf) => Ok(elf),
        None => detect_current_project_elf()?.ok_or_else(|| {
            anyhow::anyhow!(
                "No ELF file provided, and could not detect a project ELF in the current directory. Please provide an ELF file with --elf."
            )
        }),
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
