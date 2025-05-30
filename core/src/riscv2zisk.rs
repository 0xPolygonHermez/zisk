//! Converts a RISC-V program into a Zisk program.
//!
//! The input parameter is an ELF RISC-V file name, and the output parameter is a JSON Zisk ROM
//! file.  Optionally, the Zisk ROM can also be saved in PIL-friendly format or in a binary format.

use crate::{elf2rom, elf2romfile, ZiskRom};
use std::{error::Error, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AsmGenerationMethod {
    /// Generate assembly code to not even stop at chunks, nor generate trace, i.e. fast
    #[default]
    AsmFast,
    /// Generate assembly code to compute the minimal trace
    AsmMinimalTraces,
    /// Generate assembly code to compute the ROM histogram
    AsmRomHistogram,
    /// Generate assembly code to compute the main SM trace
    AsmMainTrace,
    /// Generate assembly code to stop at chunks, but do not generate any trace
    AsmChunks,
    /// Generate assembly code to compute bus op [op, a, b, mem_read_index] traces
    //AsmBusOp,
    /// Generate assembly code to compute the minimal trace, but only at the requrested chunks,
    /// e.g. [0,8,16...], [1,9,17...], etc.  This is done to distribute the minimal trace generation
    /// accross 8 processes, to increase speed and memory bus saturation.  It's called zip because
    /// one process generates the chunks that are complementary to the sum of the other processes.
    AsmZip,
    /// Generate assembly code to compute the memory operations [w/r, width, address] trace
    AsmMemOp,
    /// Generate assembly code to play a chunk from its minimal trace and collect the memory WC data
    AsmChunkPlayerMTCollectMem,
}
/// RISCV-to-ZisK struct containing the input ELF RISCV file name and the output ZISK ASM file name
pub struct Riscv2zisk {
    /// ELF RISC-V file name (input)
    pub elf_file: PathBuf,
}

impl Riscv2zisk {
    /// Creates a new Riscv2zisk struct with the provided input and output file names
    pub fn new<P: Into<PathBuf>>(elf_file: P) -> Riscv2zisk {
        Riscv2zisk { elf_file: elf_file.into() }
    }

    /// Executes the file conversion process by calling elf2romfile()
    pub fn runfile<P: Into<PathBuf>>(
        &self,
        asm_file: P,
        generation_method: AsmGenerationMethod,
        log_output: bool,
        comments: bool,
    ) -> Result<(), Box<dyn Error>> {
        elf2romfile(&self.elf_file, &asm_file.into(), generation_method, log_output, comments)
            .map_err(|e| format!("Error converting elf to assembly: {}", e).into())
    }

    /// Executes the file conversion process by calling elf2rom()
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        elf2rom(&self.elf_file)
    }
}
