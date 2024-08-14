use crate::{elf2rom, elf2romfile, ZiskRom};
use std::error::Error;

/// RISCV-to-ZisK struct containing the input ELF RISCV file name and the output ZISK ASM file name
pub struct Riscv2zisk {
    /// ELF RISC-V file name (input)
    pub elf_file: String,
    /// JSON ZISK file name (output)
    pub zisk_file: String,
    /// PIL ZISK file name (output) (optional)
    pub pil_file: String,
    /// Binary ZISK file name (output) (optional)
    pub bin_file: String,
}

impl Riscv2zisk {
    /// Creates a new Riscv2zisk struct with the provided input and output file names
    pub fn new(
        elf_file: String,
        zisk_file: String,
        pil_file: String,
        bin_file: String,
    ) -> Riscv2zisk {
        Riscv2zisk { elf_file, zisk_file, pil_file, bin_file }
    }

    /// Executes the file conversion process by calling elf2romfile()
    pub fn runfile(&self) -> Result<(), Box<dyn Error>> {
        elf2romfile(
            self.elf_file.clone(),
            self.zisk_file.clone(),
            self.pil_file.clone(),
            self.bin_file.clone(),
        )
    }

    /// Executes the file conversion process by calling elf2rom()
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        elf2rom(self.elf_file.clone())
    }
}
