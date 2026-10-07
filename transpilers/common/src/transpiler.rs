//! Converts a guest program into a Zisk program.
//!
//! The input is the contents (bytes) of a RISC-V ELF, a ziskbin ELF (a ROM prebuilt by
//! `ziskasm`) or a WebAssembly binary; the format is detected from the file's magic bytes.
//! Optionally, the Zisk ROM can also be saved in x86-64 NASM assembly format (`runfile`, the
//! `zisk-transpiler-riscv` binary).

use zisk_core::is_elf_file;
use zisk_core::is_wasm_file;
use zisk_core::ziskbin::try_elf_to_rom;
use zisk_core::AsmGenerationMethod;
use zisk_core::ZiskRom;
use zisk_core::ZiskRom2Asm;
use zisk_riscv::elf2rom;
use zisk_transpiler_wasm::wasm2rom;

use std::{error::Error, path::Path, path::PathBuf};

/// Transpiles a guest program (RISC-V ELF, ziskbin ELF or WebAssembly) into a Zisk ROM,
/// dispatching on the file's magic bytes.  This is the single seam through which every guest
/// format flows.
pub fn program2rom(bytes: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    if is_wasm_file(bytes) {
        wasm2rom(bytes)
    } else if is_elf_file(bytes).unwrap_or(false) {
        // A ziskbin ELF (e_machine == EM_ZISK) carries an already-built ZiskRom in a
        // `.ziskrom` section instead of RISC-V code; decode it directly and skip
        // transpilation. Any other ELF is a RISC-V guest.
        match try_elf_to_rom(bytes)? {
            Some(rom) => Ok(rom),
            None => elf2rom(bytes),
        }
    } else {
        Err("unrecognized guest format: expected a RISC-V ELF (\\x7fELF) or WebAssembly (\\0asm) \
             binary"
            .into())
    }
}

/// Transpiles a guest program into a Zisk ROM, like [`program2rom`], and saves the result into
/// an x86-64 assembly file.
pub fn program2romfile(
    bytes: &[u8],
    asm_file: &Path,
    generation_method: AsmGenerationMethod,
    log_output: bool,
    comments: bool,
    hints: bool,
) -> Result<(), Box<dyn Error>> {
    let rom = program2rom(bytes)?;
    ZiskRom2Asm::save_to_asm_file(&rom, asm_file, generation_method, log_output, comments, hints);

    Ok(())
}

/// Guest-to-ZisK transpiler holding the input program bytes
pub struct ZiskTranspiler<'a> {
    /// Guest program bytes (input): a RISC-V ELF, a ziskbin ELF or a WebAssembly binary
    pub program: &'a [u8],
}

/// Former name of [`ZiskTranspiler`], from when it only accepted RISC-V ELF files.
#[deprecated(note = "renamed to ZiskTranspiler")]
pub type Riscv2zisk<'a> = ZiskTranspiler<'a>;

impl<'a> ZiskTranspiler<'a> {
    /// Creates a new ZiskTranspiler struct with the provided program bytes
    pub fn new(program: &'a [u8]) -> ZiskTranspiler<'a> {
        ZiskTranspiler { program }
    }

    /// Executes the file conversion process by calling program2romfile()
    pub fn runfile<P: Into<PathBuf>>(
        &self,
        asm_file: P,
        generation_method: AsmGenerationMethod,
        log_output: bool,
        comments: bool,
        hints: bool,
    ) -> Result<(), Box<dyn Error>> {
        let asm_file = asm_file.into();
        program2romfile(self.program, &asm_file, generation_method, log_output, comments, hints)
    }

    /// Executes the conversion process, returning the Zisk ROM
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        program2rom(self.program)
    }
}
