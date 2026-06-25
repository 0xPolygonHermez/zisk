//! Converts a RISC-V program into a Zisk program.
//!
//! The input parameter is an ELF RISC-V file name, and the output parameter is a JSON Zisk ROM
//! file.  Optionally, the Zisk ROM can also be saved in x84-64 NASM assembly format.

use transpilers_common::{elf2rom, elf2romfile};
use zisk_core::ZiskRom;
use zisk_core::AsmGenerationMethod;

use std::{error::Error, path::PathBuf};

/// RISCV-to-ZisK struct containing the input ELF RISCV file name and the output ZISK ASM file name
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
        elf2romfile(self.elf, &asm_file.into(), generation_method, log_output, comments, hints)
            .map_err(|e| format!("Error converting elf to assembly: {e}").into())
    }

    /// Executes the file conversion process by calling elf2rom()
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        elf2rom(self.elf)
    }
}
