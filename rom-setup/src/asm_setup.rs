use std::{path::PathBuf, process::Command};

use anyhow::Result;
use zisk_core::{is_elf_file, Riscv2zisk};

pub fn assembly_setup(elf: &PathBuf, asm: Option<&PathBuf>) -> Result<(), anyhow::Error> {
    // Read the ELF file and check if it is a valid ELF file
    let elf_file_path = PathBuf::from(elf);
    let file_data = std::fs::read(&elf_file_path)?;

    if !is_elf_file(&file_data).unwrap_or_else(|_| panic!("Error reading ROM file")) {
        panic!("ROM file is not a valid ELF file");
    }

    // Setup the assembly file name if not provided
    let zisk_file = asm.cloned().unwrap_or_else(|| elf.with_extension("zsk"));
    let asm_file = zisk_file.with_extension("asm");

    println!("Zisk file: {}", zisk_file.to_str().unwrap());
    println!("ASM file: {}", asm_file.to_str().unwrap());

    // Convert the ELF file to Zisk format and generates an assembly file
    let rv2zk = Riscv2zisk::new(
        elf_file_path.to_str().unwrap().to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        zisk_file.to_str().unwrap().to_string(),
    );
    rv2zk.runfile().map_err(|e| anyhow::anyhow!("Error converting elf: {}", e))?;

    // Build the emulator assembly
    let status = Command::new("make")
        .arg("clean")
        .current_dir("emulator-asm")
        .status()
        .expect("Failed to run make clean");
    if !status.success() {
        eprintln!("make clean failed");
        std::process::exit(1);
    }
    let status = Command::new("make")
        .arg(format!("EMU_PATH=../{}", zisk_file.to_str().unwrap()))
        .arg(format!("OUT_PATH=../{}", asm_file.to_str().unwrap()))
        .current_dir("emulator-asm")
        .status()
        .expect("Failed to run make");

    if !status.success() {
        eprintln!("make failed");
        std::process::exit(1);
    }

    println!("Runner built successfully at: {}", asm_file.display());

    Ok(())
}
