use crate::{elf2rom, elf2romfile, ZiskRom};
use std::error::Error;

/// RISCV-to-ZisK struct containing the input ELF RISCV file name and the output ZISK ASM file name
pub struct Riscv2zisk {
    /// ELF RISC-V file name (input)
    pub elf_file: String,
    /// JSON ZISK file name (output)
    pub zisk_file: String,
}

impl Riscv2zisk {
    /// Creates a new Riscv2zisk struct with the provided input and output file names
    pub fn new(elf_file: String, zisk_file: String) -> Riscv2zisk {
        Riscv2zisk { elf_file, zisk_file }
    }

    /// Executes the file conversion process by calling elf2romfile()
    pub fn runfile(&self) -> Result<(), Box<dyn Error>> {
        elf2romfile(self.elf_file.clone(), self.zisk_file.clone())?;
        Ok(())
    }

    /// Executes the file conversion process by calling elf2rom()
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        let rom = elf2rom(self.elf_file.clone())?;
        Ok(rom)
    }
}
