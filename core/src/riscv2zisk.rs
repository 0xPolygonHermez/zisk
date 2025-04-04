//! Converts a RISC-V program into a Zisk program.
//!
//! The input parameter is an ELF RISC-V file name, and the output parameter is a JSON Zisk ROM
//! file.  Optionally, the Zisk ROM can also be saved in PIL-friendly format or in a binary format.

use crate::{elf2rom, elf2romfile, ZiskRom};
use std::{error::Error, path::PathBuf};

pub enum AsmGenerationMethod {
    /// Generate assembly code to compute the minimal traces
    AsmMinimalTraces,
    /// Generate assembly code to compute the ROM histogram
    AsmRomHistogram,
}
/// RISCV-to-ZisK struct containing the input ELF RISCV file name and the output ZISK ASM file name
pub struct Riscv2zisk {
    /// ELF RISC-V file name (input)
    pub elf_file: PathBuf,
    /// Assembly i86-64 file name (output)
    pub asm_file: Option<PathBuf>,
}

impl Riscv2zisk {
    /// Creates a new Riscv2zisk struct with the provided input and output file names
    pub fn new<P: Into<PathBuf>>(elf_file: P, asm_file: Option<P>) -> Riscv2zisk {
        Riscv2zisk { elf_file: elf_file.into(), asm_file: asm_file.map(Into::into) }
    }

    /// Executes the file conversion process by calling elf2romfile()
    pub fn runfile(&self, generation_method: AsmGenerationMethod) -> Result<(), Box<dyn Error>> {
        elf2romfile(&self.elf_file, self.asm_file.as_deref(), generation_method)
            .map_err(|e| format!("Error converting elf to assembly: {}", e).into())
    }

    /// Executes the file conversion process by calling elf2rom()
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        elf2rom(&self.elf_file)
    }
}
