use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Result;
use zisk_core::{is_elf_file, Riscv2zisk};

pub fn assembly_setup(
    elf: &PathBuf,
    output_path: &Path,
    verbose: bool,
) -> Result<(), anyhow::Error> {
    // Read the ELF file and check if it is a valid ELF file
    let elf_file_path = PathBuf::from(elf);
    let file_data = std::fs::read(&elf_file_path)?;

    if !is_elf_file(&file_data).unwrap_or_else(|_| panic!("Error reading ROM file")) {
        panic!("ROM file is not a valid ELF file");
    }

    let filename = elf.file_name().unwrap().to_string_lossy().into_owned();

    let base_path = output_path.join(filename);

    let zisk_file = base_path.with_extension("zisk");
    let asm_file = base_path.with_extension("asm");

    // Convert the ELF file to Zisk format and generates an assembly file
    let rv2zk = Riscv2zisk::new(
        elf_file_path.to_str().unwrap().to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        zisk_file.to_str().unwrap().to_string(),
    );
    rv2zk.runfile(verbose).map_err(|e| anyhow::anyhow!("Error converting elf: {}", e))?;

    // Build the emulator assembly
    let status = Command::new("make")
        .arg("clean")
        .current_dir("emulator-asm")
        .stdout(if verbose { Stdio::inherit() } else { Stdio::null() })
        .stderr(if verbose { Stdio::inherit() } else { Stdio::null() })
        .status()
        .expect("Failed to run make clean");

    if !status.success() {
        eprintln!("make clean failed");
        std::process::exit(1);
    }

    let status = Command::new("make")
        .arg(format!("EMU_PATH={}", zisk_file.to_str().unwrap()))
        .arg(format!("OUT_PATH={}", asm_file.to_str().unwrap()))
        .current_dir("emulator-asm")
        .stdout(if verbose { Stdio::inherit() } else { Stdio::null() })
        .stderr(if verbose { Stdio::inherit() } else { Stdio::null() })
        .status()
        .expect("Failed to run make");

    if !status.success() {
        eprintln!("make failed");
        std::process::exit(1);
    }

    Ok(())
}
