//! Converts a RISC-V program into a Zisk program.
//!
//! The input parameter is an ELF RISC-V file name, and the output parameter is a JSON Zisk ROM
//! file.  Optionally, the Zisk ROM can also be saved in x84-64 NASM assembly format.

use crate::{elf2rom, elf2romfile, ZiskRom};
use std::{error::Error, path::PathBuf};

/// ZisK Emulator can be executed in assembly to get the maximum performance
/// in the first sequential emulation, and also is some subsequent parallel tasks.
///
/// ROM histogram contains a counter per program counter that is incremented every time that
/// instruction is executed.  It is generated in one single, sequential emulation.
///
/// Mem reads contain all the memory reads donde during a chunk of the emulation.  Mem reads chunks
/// are generated sequentially, and consumed in parallel after the first chunk is ready to generate
/// the main AIR traces.
///
/// Mem trace contains a record of all the memory operations: step, r/w, address, width, write
/// value, etc.  Mem trace is generated sequentially in chunks, which are consumed in parallel in C
/// to generate the memory AIR plan and AIR traces.
///
/// ```text
///                 /-> [ASM seq] -> ROM Histogram
///                /
/// RISC-V -> ZisK ---> [ASM seq] -> Mem Reads chunks -> [ASM par chunk player] -> Main Trace
///                \
///                 \-> [ASM seq] -> Mem Trace chunks -> [  C par chunk player] -> Mem Plan & Trace
/// ```
///
/// Other meaningful assembly emulation methods used for performance investigation include:
/// - Fast: Does not generate any trace, but simply emulates the program.  It is the fastest method.
/// - Chunks: Stops every chunk-size steps, without generating traces.
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
    /// Generate assembly code to compute the minimal trace, but only at the requested chunks,
    /// e.g. [0,8,16...], [1,9,17...], etc.  This is done to distribute the minimal trace generation
    /// across 8 processes, to increase speed and memory bus saturation.  It's called zip because
    /// one process generates the chunks that are complementary to the sum of the other processes.
    AsmZip,
    /// Generate assembly code to compute the memory operations [w/r, width, address] trace
    AsmMemOp,
    /// Generate assembly code to play a chunk from its minimal trace and collect the memory WC data
    AsmChunkPlayerMTCollectMem,
    /// Generate assembly code to compute the memory reads trace
    AsmMemReads,
    /// Generate assembly code to play a chunk from its memory reads trace and collect the main WC
    /// data
    AsmChunkPlayerMemReadsCollectMain,
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
            .map_err(|e| format!("Error converting elf to assembly: {e}").into())
    }

    /// Executes the file conversion process by calling elf2rom()
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        elf2rom(&self.elf_file)
    }
}
