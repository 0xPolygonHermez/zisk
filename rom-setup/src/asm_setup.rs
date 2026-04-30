use anyhow::Context;
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
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

fn asm_file_base(name: &str, hash: &str, hints: bool, deps_hash: &str) -> String {
    let prefix = if name != hash { format!("{name}-{hash}") } else { hash.to_string() };
    // `deps_hash` keys the cache on the contents of libziskc.a / libziskclib.a /
    // ziskfloat.elf so a workspace rebuild of any of them produces a different
    // cache filename and the emu binary is regenerated against the new libs.
    // Empty in installed mode where the libs don't change between runs.
    let prefix = if deps_hash.is_empty() { prefix } else { format!("{prefix}-d{deps_hash}") };
    if hints {
        format!("{prefix}-hints")
    } else {
        prefix
    }
}

/// Resolve the lib paths that the emulator-asm Makefile will link against,
/// in the same order as `compute_make_overrides`. Returns an empty vec in
/// installed mode (no `<workspace>/target/`), where libs don't change between
/// runs and a deps hash isn't needed.
fn resolve_link_inputs(emulator_asm_path: &Path) -> Vec<PathBuf> {
    let Some(parent) = emulator_asm_path.parent() else { return Vec::new() };
    let target_dir = parent.join("target");
    if !target_dir.exists() {
        return Vec::new();
    }
    vec![
        target_dir.join("zisk-libs").join("libziskc.a"),
        target_dir.join("release").join("libziskclib.a"),
        target_dir.join("zisk-libs").join("ziskfloat.elf"),
    ]
}

/// Hash the link-time inputs (libziskc.a, libziskclib.a, ziskfloat.elf) so the
/// asm cache key changes whenever any of them is rebuilt. Returns an empty
/// string when there's nothing to hash (installed mode), letting the caller
/// fall back to the legacy elf-only cache key.
fn compute_deps_hash(emulator_asm_path: &Path) -> String {
    let inputs = resolve_link_inputs(emulator_asm_path);
    if inputs.is_empty() {
        return String::new();
    }
    let mut hasher = blake3::Hasher::new();
    for path in &inputs {
        // Hash the basename + a separator + length + content so two inputs of
        // the same content but different roles can never collide.
        let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
        hasher.update(name.as_bytes());
        hasher.update(&[0u8]);
        match std::fs::read(path) {
            Ok(bytes) => {
                hasher.update(&(bytes.len() as u64).to_le_bytes());
                hasher.update(&bytes);
            }
            Err(_) => {
                // Missing input — record the absence so a later present-state
                // produces a different hash. The actual link will fail loudly
                // downstream if the lib really is missing.
                hasher.update(b"<missing>");
            }
        }
        hasher.update(&[0xffu8]);
    }
    let digest = hasher.finalize();
    // 12 hex chars (48 bits) is plenty to distinguish lib versions.
    digest.to_hex().as_str()[..12].to_string()
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
    let base = compute_asm_basename(elf_name, &elf_hash, hints);

    Ok(vec![
        output_path.join(format!("{base}-mt.bin")),
        output_path.join(format!("{base}-rh.bin")),
        output_path.join(format!("{base}-mo.bin")),
    ])
}

/// Build the cache-base filename for the asm emulator binaries — the part
/// before `-{mt,rh,mo}.bin`. This is the **canonical** entry point: any caller
/// constructing a path to an asm binary (producer or consumer) must route
/// through here so the deps_hash segment stays consistent.
pub fn compute_asm_basename(elf_name: &str, elf_hash: &str, hints: bool) -> String {
    // In installed mode `resolve_emulator_asm` succeeds but
    // `compute_deps_hash` returns an empty string — we silently fall back to
    // the legacy elf-only key in that case.
    let deps_hash = match resolve_emulator_asm() {
        Ok(p) => compute_deps_hash(&p),
        Err(_) => String::new(),
    };
    asm_file_base(elf_name, elf_hash, hints, &deps_hash)
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

    let emulator_asm_path = resolve_emulator_asm()?;
    let deps_hash = compute_deps_hash(&emulator_asm_path);
    let base = asm_file_base(elf_name, &elf_hash, hints, &deps_hash);

    let bin_mt_file = output_path.join(format!("{base}-mt.bin"));
    let bin_rh_file = output_path.join(format!("{base}-rh.bin"));
    let bin_mo_file = output_path.join(format!("{base}-mo.bin"));

    // Decide where the Makefile should drop its intermediates and find the
    // ziskc / ziskclib static libs. In workspace mode we point everything at
    // `<workspace>/target/...` so `cargo clean` removes it; in installed mode
    // we leave the Makefile's defaults alone (they resolve relative to the
    // install dir).
    let make_dir_overrides = compute_make_overrides(&emulator_asm_path);

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
        let mut clean_cmd = Command::new("make");
        clean_cmd.arg("clean");
        for arg in &make_dir_overrides {
            clean_cmd.arg(arg);
        }
        let status = clean_cmd
            .current_dir(emulator_asm_path)
            .stdout(if verbose { Stdio::inherit() } else { Stdio::null() })
            .stderr(if verbose { Stdio::inherit() } else { Stdio::null() })
            .status()
            .context("Failed to execute 'make clean' command")?;

        if !status.success() {
            anyhow::bail!("'make clean' failed with exit code: {:?}", status.code());
        }

        let out_file_str = file.to_str().context("Failed to convert output file path to string")?;

        let mut build_cmd = Command::new("make");
        build_cmd
            .arg(format!("EMU_PATH={}", asm_file_str))
            .arg(format!("OUT_PATH={}", out_file_str))
            .arg(format!("TRACE_TARGET={trace_target}"));
        for arg in &make_dir_overrides {
            build_cmd.arg(arg);
        }
        let status = build_cmd
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

/// Compute the `make` arg overrides for `BUILD_DIR`, `ZISKC_LIB_DIR`, and
/// `ZISKCLIB_LIB_DIR`. In a workspace build we redirect them under `target/`
/// so `cargo clean` reaches them; in installed mode we return an empty list
/// so the Makefile defaults stand.
fn compute_make_overrides(emulator_asm_path: &Path) -> Vec<String> {
    let Some(parent) = emulator_asm_path.parent() else { return Vec::new() };
    let target_dir = parent.join("target");
    if !target_dir.exists() {
        return Vec::new();
    }
    let build_dir = target_dir.join("zisk-emulator-asm-build");
    let ziskc_lib_dir = target_dir.join("zisk-libs");
    let ziskclib_lib_dir = target_dir.join("release");
    vec![
        format!("BUILD_DIR={}", build_dir.display()),
        format!("ZISKC_LIB_DIR={}", ziskc_lib_dir.display()),
        format!("ZISKCLIB_LIB_DIR={}", ziskclib_lib_dir.display()),
    ]
}
