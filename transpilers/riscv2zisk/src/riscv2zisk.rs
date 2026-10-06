//! Converts a RISC-V program into a Zisk program.
//!
//! The input parameter is the contents (bytes) of an ELF RISC-V file.
//! Optionally, the Zisk ROM can also be saved in x86-64 NASM assembly format.

use zisk_core::AsmGenerationMethod;
use zisk_core::ZiskRom;
use zisk_transpiler_common::{elf2rom, elf2romfile};

use std::{error::Error, path::PathBuf};

/// RISCV-to-ZisK struct containing the input ELF RISCV file data
pub struct Riscv2zisk<'a> {
    /// ELF RISC-V file bytes (input)
    pub elf: &'a [u8],
}

impl<'a> Riscv2zisk<'a> {
    /// Creates a new Riscv2zisk struct with the provided ELF bytes
    pub fn new(elf: &'a [u8]) -> Riscv2zisk<'a> {
        Riscv2zisk { elf }
    }

    /// Executes the file conversion process by calling elf2romfile()
    pub fn runfile<P: Into<PathBuf>>(
        &self,
        asm_file: P,
        generation_method: AsmGenerationMethod,
        log_output: bool,
        comments: bool,
        hints: bool,
    ) -> Result<(), Box<dyn Error>> {
        let asm_file = asm_file.into();
        elf2romfile(self.elf, &asm_file, generation_method, log_output, comments, hints)
    }

    /// Executes the file conversion process by calling elf2rom()
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        elf2rom(self.elf)
    }
}
