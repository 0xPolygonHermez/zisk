use anyhow::Context;
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use zisk_common::ZiskPaths;
use zisk_core::{is_elf_file, AsmGenerationMethod, Riscv2zisk};

use crate::get_elf_data_hash;
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

#[derive(Clone, Copy, Debug)]
pub enum EmulatorAsmSource {
    Workspace,
    Installed,
}

pub fn resolve_emulator_asm() -> Result<(PathBuf, EmulatorAsmSource)> {
    if std::env::var_os("ZISK_USE_INSTALLED").is_some_and(|v| !v.is_empty()) {
        let installed_asm_path = ZiskPaths::global().emulator_asm.clone();
        tracing::debug!(
            "ZISK_USE_INSTALLED set, using installed emulator-asm at: {}",
            installed_asm_path.display()
        );
        if !installed_asm_path.exists() {
            anyhow::bail!("emulator-asm directory not found at: {}", installed_asm_path.display());
        }
        return Ok((installed_asm_path, EmulatorAsmSource::Installed));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root =
        if manifest_dir.exists() { find_workspace_root(&manifest_dir) } else { None };

    let cargo_available = Command::new("cargo")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    let workspace_choice = workspace_root.as_ref().and_then(|root| {
        let candidate = root.join("emulator-asm");
        let ziskclib = root.join("ziskclib");
        (cargo_available && candidate.exists() && ziskclib.exists()).then_some(candidate)
    });

    let (emulator_asm_path, source) = if let Some(path) = workspace_choice {
        tracing::debug!("Using emulator-asm from workspace: {}", path.display());
        (path, EmulatorAsmSource::Workspace)
    } else {
        let installed_asm_path = ZiskPaths::global().emulator_asm.clone();
        let reason = if !cargo_available {
            "cargo not available"
        } else if workspace_root.is_none() {
            "no workspace found"
        } else {
            "workspace missing emulator-asm or ziskclib source"
        };
        tracing::debug!(
            "Using installed emulator-asm at {} ({reason})",
            installed_asm_path.display()
        );
        (installed_asm_path, EmulatorAsmSource::Installed)
    };

    if !emulator_asm_path.exists() {
        anyhow::bail!("emulator-asm directory not found at: {}", emulator_asm_path.display());
    }

    Ok((emulator_asm_path, source))
}

pub fn ensure_ziskclib(emu_dir: &Path, source: EmulatorAsmSource) -> Result<()> {
    let emulator_parent =
        emu_dir.parent().context("Failed to get parent directory of emulator-asm")?;

    let target_lib_path = match source {
        EmulatorAsmSource::Installed => ZiskPaths::global().libziskclib.clone(),
        EmulatorAsmSource::Workspace => emulator_parent.join("target/release/libziskclib.a"),
    };

    tracing::debug!("Looking for ziskclib at: {}", target_lib_path.display());

    match source {
        EmulatorAsmSource::Workspace => {
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
        }
        EmulatorAsmSource::Installed => {
            if !target_lib_path.exists() {
                anyhow::bail!(
                    "Pre-built libziskclib.a not found at: {}\nPlease ensure zisk is properly installed via ziskup",
                    target_lib_path.display()
                );
            }
            tracing::debug!("Using existing ziskclib at: {}", target_lib_path.display());
        }
    }

    Ok(())
}

fn asm_file_base(name: &str, hash: &str, hints: bool) -> String {
    let prefix = if name != hash { format!("{name}-{hash}") } else { hash.to_string() };
    if hints {
        format!("{prefix}-hints")
    } else {
        prefix
    }
}

/// Get the paths to all assembly binary files for a given ELF and output path
pub fn get_assembly_file_paths(
    elf: &Path,
    output_path: &Path,
    hints: bool,
) -> Result<Vec<PathBuf>> {
    let elf_hash = get_elf_data_hash_from_path(elf)?;
    let elf_name = elf
        .file_stem()
        .context("Failed to extract file stem from ELF path")?
        .to_str()
        .context("Failed to convert ELF file stem to string")?;
    let base = asm_file_base(elf_name, &elf_hash, hints);

    Ok(vec![
        output_path.join(format!("{base}-mt.bin")),
        output_path.join(format!("{base}-rh.bin")),
        output_path.join(format!("{base}-mo.bin")),
    ])
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
    let elf_hash = get_elf_data_hash(elf);

    if !is_elf_file(elf).context("Error reading ROM file")? {
        anyhow::bail!("ROM file is not a valid ELF file");
    }

    let base = asm_file_base(elf_name, &elf_hash, hints);

    let bin_mt_file = output_path.join(format!("{base}-mt.bin"));
    let bin_rh_file = output_path.join(format!("{base}-rh.bin"));
    let bin_mo_file = output_path.join(format!("{base}-mo.bin"));

    let (emulator_asm_path, asm_source) = resolve_emulator_asm()?;
    ensure_ziskclib(&emulator_asm_path, asm_source)?;

    let emulator_asm_path =
        emulator_asm_path.to_str().context("Failed to convert emulator-asm path to string")?;

    for (file, gen_method, trace_target) in [
        (bin_mt_file, AsmGenerationMethod::AsmMinimalTraces, "MT"),
        (bin_rh_file, AsmGenerationMethod::AsmRomHistogram, "RH"),
        (bin_mo_file, AsmGenerationMethod::AsmMemOp, "MO"),
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
            .arg(format!("TRACE_TARGET={trace_target}"))
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
