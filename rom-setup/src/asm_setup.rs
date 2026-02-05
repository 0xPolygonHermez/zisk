use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Result;
use zisk_core::{is_elf_file, AsmGenerationMethod, Riscv2zisk};

use crate::get_elf_data_hash_from_path;

/// Check if all assembly binary files exist for a given ELF and output path
pub fn assembly_files_exist(elf: &Path, output_path: &Path, hints: bool) -> Result<bool> {
    let elf_hash = get_elf_data_hash_from_path(elf)?;

    let stem = elf.file_stem().unwrap().to_str().unwrap();
    let stem = if hints { format!("{stem}-hints") } else { stem.to_string() };
    let new_filename = format!("{stem}-{elf_hash}.tmp");
    let base_path = output_path.join(new_filename);
    let file_stem = base_path.file_stem().unwrap().to_str().unwrap();

    let bin_mt_file = format!("{file_stem}-mt.bin");
    let bin_mt_file = base_path.with_file_name(bin_mt_file);

    let bin_rh_file = format!("{file_stem}-rh.bin");
    let bin_rh_file = base_path.with_file_name(bin_rh_file);

    let bin_mo_file = format!("{file_stem}-mo.bin");
    let bin_mo_file = base_path.with_file_name(bin_mo_file);

    Ok(bin_mt_file.exists() && bin_rh_file.exists() && bin_mo_file.exists())
}

pub fn gen_assembly(
    _elf: &Path,
    _zisk_path: &Option<PathBuf>,
    _output_dir: &Option<PathBuf>,
    _hints: bool,
    _verbose: bool,
) -> Result<(), anyhow::Error> {
    // Assembly setup is not needed on macOS due to the lack of support for assembly generation.
    #[cfg(not(target_os = "macos"))]
    {
        let output_path = crate::get_output_path(_output_dir)?;
        let elf_hash = get_elf_data_hash_from_path(_elf)?;

        tracing::info!("Computing assembly setup");
        let zisk_path = crate::get_zisk_path(_zisk_path.as_ref());
        _generate_assembly(_elf, &elf_hash, &zisk_path, output_path.as_path(), _hints, _verbose)?;
        tracing::info!("Assembly setup generated at {}", output_path.display());
    }
    Ok(())
}

fn _generate_assembly(
    elf: &Path,
    elf_hash: &str,
    zisk_path: &Path,
    output_path: &Path,
    hints: bool,
    verbose: bool,
) -> Result<(), anyhow::Error> {
    // Read the ELF file and check if it is a valid ELF file
    let elf_file_path = PathBuf::from(elf);
    let file_data = std::fs::read(&elf_file_path)?;

    if !is_elf_file(&file_data).unwrap_or_else(|_| panic!("Error reading ROM file")) {
        panic!("ROM file is not a valid ELF file");
    }

    let stem = elf.file_stem().unwrap().to_str().unwrap();
    let stem = if hints { format!("{stem}-hints") } else { stem.to_string() };
    let new_filename = format!("{stem}-{elf_hash}.tmp");
    let base_path = output_path.join(new_filename);
    let file_stem = base_path.file_stem().unwrap().to_str().unwrap();

    let bin_mt_file = format!("{file_stem}-mt.bin");
    let bin_mt_file = base_path.with_file_name(bin_mt_file);

    let bin_rh_file = format!("{file_stem}-rh.bin");
    let bin_rh_file = base_path.with_file_name(bin_rh_file);

    let bin_mo_file = format!("{file_stem}-mo.bin");
    let bin_mo_file = base_path.with_file_name(bin_mo_file);

    [
        (bin_mt_file, AsmGenerationMethod::AsmMinimalTraces),
        (bin_rh_file, AsmGenerationMethod::AsmRomHistogram),
        (bin_mo_file, AsmGenerationMethod::AsmMemOp),
    ]
    .iter()
    .for_each(|(file, gen_method)| {
        let asm_file = file.with_extension("asm");
        // Convert the ELF file to Zisk format and generates an assembly file
        let rv2zk = Riscv2zisk::new(&file_data);
        rv2zk
            .runfile(asm_file.to_str().unwrap().to_string(), *gen_method, false, false, hints)
            .expect("Error converting elf to assembly");

        let emulator_asm_path = zisk_path.join("emulator-asm");
        let emulator_asm_path = emulator_asm_path.to_str().unwrap();

        // Build the emulator assembly
        let status = Command::new("make")
            .arg("clean")
            .current_dir(emulator_asm_path)
            .stdout(if verbose { Stdio::inherit() } else { Stdio::null() })
            .stderr(if verbose { Stdio::inherit() } else { Stdio::null() })
            .status()
            .expect("Failed to run make clean");

        if !status.success() {
            eprintln!("make clean failed");
            std::process::exit(1);
        }

        let status = Command::new("make")
            .arg(format!("EMU_PATH={}", asm_file.to_str().unwrap()))
            .arg(format!("OUT_PATH={}", file.to_str().unwrap()))
            .current_dir(emulator_asm_path)
            .stdout(if verbose { Stdio::inherit() } else { Stdio::null() })
            .stderr(if verbose { Stdio::inherit() } else { Stdio::null() })
            .status()
            .expect("Failed to run make");

        if !status.success() {
            eprintln!("make failed");
            std::process::exit(1);
        }
    });

    Ok(())
}
