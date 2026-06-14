//! Converts a guest program into a Zisk program.
//!
//! The input parameter is the contents (bytes) of an ELF RISC-V file or of a WebAssembly binary.
//! Optionally, the Zisk ROM can also be saved in x86-64 NASM assembly format.

use zisk_core::is_elf_file;
use zisk_core::is_wasm_file;
use zisk_core::AsmGenerationMethod;
use zisk_core::ZiskRom;
use zisk_transpiler_common::{elf2rom, elf2romfile};
use zisk_transpiler_wasm::wasm2rom;

use std::{error::Error, path::PathBuf};

/// Transpiles a guest program (RISC-V ELF or WebAssembly) into a Zisk ROM, dispatching on the
/// file's magic bytes.  This is the single seam through which both guest machines flow.
pub fn program2rom(bytes: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    if is_wasm_file(bytes) {
        wasm2rom(bytes)
    } else if is_elf_file(bytes).unwrap_or(false) {
        elf2rom(bytes)
    } else {
        Err("unrecognized guest format: expected a RISC-V ELF (\\x7fELF) or WebAssembly (\\0asm) \
             binary"
            .into())
    }
}

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

    /// Executes the file conversion process.  Despite the historical name, this accepts either a
    /// RISC-V ELF or a WebAssembly guest and dispatches on the file's magic bytes.
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        program2rom(self.elf)
    }
}
