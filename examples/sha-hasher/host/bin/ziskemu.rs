use anyhow::Result;
use std::path::PathBuf;
use zisk_sdk::{EmuOptions, elf_path, ziskemu, ZiskStdin, ZiskIO, ElfBinaryFromFile};

fn main() -> Result<()> {
    let elf_path = elf_path!("sha-hasher-guest");
    println!("Loading ELF binary from path: {}", elf_path);
    let elf = ElfBinaryFromFile::new(&PathBuf::from(elf_path), false)?;

    let current_dir = std::env::current_dir()?;
    let stdin =
        ZiskStdin::from_file(current_dir.join("sha-hasher/host/tmp/verify_constraints_input.bin"))?;

    let n: u32 = stdin.read()?;
    println!("Input prepared: {} iterations", n);

    println!("Running ZisK Emulator...");
    let emu_options = EmuOptions { stats: true, ..EmuOptions::default() };
    ziskemu(&elf, stdin, &emu_options)?;
    println!("ZisK Emulator completed successfully!");

    Ok(())
}
