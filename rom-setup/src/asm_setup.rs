use anyhow::Context;
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use zisk_core::{is_elf_file, AsmGenerationMethod, Riscv2zisk};

use crate::get_elf_data_hash_from_path;

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);

    while let Some(dir) = current {
        let cargo_toml = dir.join("Cargo.toml");

        if cargo_toml.exists() {
            if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                if contents.contains("[workspace]") {
                    return Some(dir.to_path_buf());
                }
            }
        }

        current = dir.parent();
    }

    None
}

pub fn resolve_emulator_asm() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let workspace_root =
        if manifest_dir.exists() { find_workspace_root(&manifest_dir) } else { None };

    let cargo_available = Command::new("cargo")
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    // Check if we can build from workspace (need both cargo and workspace with ziskclib)
    let can_build_from_workspace = cargo_available
        && if let Some(ref root) = workspace_root {
            let candidate = root.join("emulator-asm");
            let ziskclib_path = root.join("ziskclib");
            candidate.exists() && ziskclib_path.exists()
        } else {
            false
        };

    let installed_path = crate::get_default_zisk_path();
    let installed_asm_path = installed_path.join("zisk/emulator-asm");

    let emulator_asm_path = if can_build_from_workspace {
        let candidate = workspace_root.unwrap().join("emulator-asm");
        tracing::debug!("Using emulator-asm from workspace: {}", candidate.display());
        candidate
    } else {
        if !cargo_available {
            tracing::debug!(
                "Cargo not available, using installed path: {}",
                installed_asm_path.display()
            );
        } else if workspace_root.is_none() {
            tracing::debug!(
                "No workspace found, using installed path: {}",
                installed_asm_path.display()
            );
        } else {
            tracing::debug!(
                "Workspace missing ziskclib source, using installed path: {}",
                installed_asm_path.display()
            );
        }

        installed_asm_path.clone()
    };

    tracing::info!("Looking for emulator-asm at: {}", emulator_asm_path.display());

    if !emulator_asm_path.exists() {
        anyhow::bail!("emulator-asm directory not found at: {}", emulator_asm_path.display());
    }

    let emulator_parent =
        emulator_asm_path.parent().context("Failed to get parent directory of emulator-asm")?;
    let ziskclib_path = emulator_parent.join("ziskclib");

    let target_lib_path = if emulator_asm_path == installed_asm_path {
        // For installed path, look in .zisk/bin/
        installed_path.join("bin").join("libziskclib.a")
    } else {
        // For workspace builds, look in target/release/
        emulator_parent.join("target/release/libziskclib.a")
    };

    tracing::info!("Looking for ziskclib at: {}", target_lib_path.display());

    // Only try to build if cargo is available and ziskclib source exists
    if cargo_available && ziskclib_path.exists() {
        tracing::debug!("Found ziskclib at: {}", ziskclib_path.display());
        tracing::debug!("Building ziskclib...");

        let output = Command::new("cargo")
            .args(["build", "--release", "-p", "ziskclib"])
            .current_dir(emulator_parent)
            .output()
            .context("Failed to execute cargo build for ziskclib")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!("Failed to build ziskclib:\nstdout: {}\nstderr: {}", stdout, stderr);
        }

        if !target_lib_path.exists() {
            anyhow::bail!(
                "ziskclib build succeeded but library not found at: {}",
                target_lib_path.display()
            );
        }

        tracing::debug!("ziskclib built successfully at: {}", target_lib_path.display());
    } else {
        if !target_lib_path.exists() {
            if emulator_asm_path == installed_path {
                anyhow::bail!(
                    "Pre-built libziskclib.a not found at: {}\nPlease ensure zisk is properly installed",
                    target_lib_path.display()
                );
            } else if cargo_available {
                anyhow::bail!(
                    "libziskclib.a not found at: {}\nziskclib directory not found at: {}\nCannot build or locate ziskclib library",
                    target_lib_path.display(),
                    ziskclib_path.display()
                );
            } else {
                anyhow::bail!(
                    "libziskclib.a not found at: {}\nCargo not available for building from source\nConsider using the installed version instead",
                    target_lib_path.display()
                );
            }
        }
        tracing::debug!("Using existing ziskclib at: {}", target_lib_path.display());
    }

    Ok(emulator_asm_path)
}

/// Get the paths to all assembly binary files for a given ELF and output path
pub fn get_assembly_file_paths(
    elf: &Path,
    output_path: &Path,
    hints: bool,
) -> Result<Vec<PathBuf>> {
    let elf_hash = get_elf_data_hash_from_path(elf)?;

    let stem = elf
        .file_stem()
        .context("Failed to extract file stem from ELF path")?
        .to_str()
        .context("Failed to convert ELF file stem to string")?;
    let stem = if hints { format!("{stem}-hints") } else { stem.to_string() };
    let new_filename = format!("{stem}-{elf_hash}.tmp");
    let base_path = output_path.join(new_filename);
    let file_stem = base_path
        .file_stem()
        .context("Failed to extract file stem from base path")?
        .to_str()
        .context("Failed to convert file stem to string")?;

    let bin_mt_file = format!("{file_stem}-mt.bin");
    let bin_mt_file = base_path.with_file_name(bin_mt_file);

    let bin_rh_file = format!("{file_stem}-rh.bin");
    let bin_rh_file = base_path.with_file_name(bin_rh_file);

    let bin_mo_file = format!("{file_stem}-mo.bin");
    let bin_mo_file = base_path.with_file_name(bin_mo_file);

    Ok(vec![bin_mt_file, bin_rh_file, bin_mo_file])
}

/// Check if all assembly binary files exist for a given ELF and output path
pub fn assembly_files_exist(elf: &Path, output_path: &Path, hints: bool) -> Result<bool> {
    let files = get_assembly_file_paths(elf, output_path, hints)?;
    Ok(files.iter().all(|f| f.exists()))
}

pub fn gen_assembly(
    _elf: &Path,
    _output_dir: &Option<PathBuf>,
    _hints: bool,
    _verbose: bool,
) -> Result<(), anyhow::Error> {
    // Assembly setup is not needed on macOS due to the lack of support for assembly generation.
    #[cfg(not(target_os = "macos"))]
    {
        let output_path = crate::get_output_path(_output_dir)?;
        let elf_data =
            std::fs::read(_elf).with_context(|| format!("Error reading ELF file: {_elf:?}"))?;
        let stem = _elf
            .file_stem()
            .context("Failed to extract file stem from ELF path")?
            .to_str()
            .context("Failed to convert ELF file stem to string")?;
        tracing::info!("Computing assembly setup");
        generate_assembly(&elf_data, stem, output_path.as_path(), _hints, _verbose)?;
        tracing::info!("Assembly setup generated at {}", output_path.display());
    }
    Ok(())
}

pub fn generate_assembly(
    elf: &[u8],
    elf_name: &str,
    output_path: &Path,
    hints: bool,
    verbose: bool,
) -> Result<(), anyhow::Error> {
    let elf_hash = blake3::hash(elf).to_hex().to_string();

    if !is_elf_file(elf).context("Error reading ROM file")? {
        anyhow::bail!("ROM file is not a valid ELF file");
    }

    let stem = if hints { format!("{elf_name}-hints") } else { elf_name.to_string() };
    let new_filename = format!("{stem}-{elf_hash}.tmp");
    let base_path = output_path.join(new_filename);
    let file_stem = base_path
        .file_stem()
        .context("Failed to extract file stem from base path")?
        .to_str()
        .context("Failed to convert file stem to string")?;

    let bin_mt_file = format!("{file_stem}-mt.bin");
    let bin_mt_file = base_path.with_file_name(bin_mt_file);

    let bin_rh_file = format!("{file_stem}-rh.bin");
    let bin_rh_file = base_path.with_file_name(bin_rh_file);

    let bin_mo_file = format!("{file_stem}-mo.bin");
    let bin_mo_file = base_path.with_file_name(bin_mo_file);

    let emulator_asm_path = resolve_emulator_asm()?;

    let emulator_asm_path =
        emulator_asm_path.to_str().context("Failed to convert emulator-asm path to string")?;

    for (file, gen_method) in [
        (bin_mt_file, AsmGenerationMethod::AsmMinimalTraces),
        (bin_rh_file, AsmGenerationMethod::AsmRomHistogram),
        (bin_mo_file, AsmGenerationMethod::AsmMemOp),
    ] {
        let asm_file = file.with_extension("asm");
        // Convert the ELF file to Zisk format and generates an assembly file
        let rv2zk = Riscv2zisk::new(elf);
        let asm_file_str =
            asm_file.to_str().context("Failed to convert asm_file path to string")?;
        rv2zk
            .runfile(asm_file_str.to_string(), gen_method, false, false, hints)
            .map_err(|e| anyhow::anyhow!("Error converting ELF to assembly: {}", e))?;

        // Build the emulator assembly
        let status = Command::new("make")
            .arg("clean")
            .current_dir(emulator_asm_path)
            .stdout(if verbose { Stdio::inherit() } else { Stdio::null() })
            .stderr(if verbose { Stdio::inherit() } else { Stdio::null() })
            .status()
            .context("Failed to execute 'make clean' command")?;

        if !status.success() {
            anyhow::bail!("'make clean' failed with exit code: {:?}", status.code());
        }

        let out_file_str = file.to_str().context("Failed to convert output file path to string")?;

        let status = Command::new("make")
            .arg(format!("EMU_PATH={}", asm_file_str))
            .arg(format!("OUT_PATH={}", out_file_str))
            .current_dir(emulator_asm_path)
            .stdout(if verbose { Stdio::inherit() } else { Stdio::null() })
            .stderr(if verbose { Stdio::inherit() } else { Stdio::null() })
            .status()
            .context("Failed to execute 'make' command")?;

        if !status.success() {
            anyhow::bail!("'make' failed with exit code: {:?}", status.code());
        }
    }

    Ok(())
}
