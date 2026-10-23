//! `Riscv2zisk`, the original name of the guest-to-ZisK transpiler, with its original API.
//!
//! Deprecated: use `zisk_transpiler_common::ZiskTranspiler`, which this delegates to.

use zisk_core::AsmGenerationMethod;
use zisk_core::ZiskRom;
use zisk_transpiler_common::ZiskTranspiler;

use std::{error::Error, path::PathBuf};

/// Transpiles a guest program (RISC-V ELF, ziskbin ELF or WebAssembly) into a Zisk ROM.
#[deprecated(since = "1.4.0", note = "use zisk_transpiler_common::program2rom")]
pub fn program2rom(bytes: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    zisk_transpiler_common::program2rom(bytes)
}

/// RISCV-to-ZisK struct containing the input program bytes.  Despite the name, it accepts every
/// guest format `program2rom` does: RISC-V ELF, ziskbin ELF and WebAssembly.
#[deprecated(since = "1.4.0", note = "use zisk_transpiler_common::ZiskTranspiler")]
pub struct Riscv2zisk<'a> {
    /// Guest program bytes (input)
    pub elf: &'a [u8],
}

#[allow(deprecated)]
impl<'a> Riscv2zisk<'a> {
    /// Creates a new Riscv2zisk struct with the provided program bytes
    pub fn new(elf: &'a [u8]) -> Riscv2zisk<'a> {
        Riscv2zisk { elf }
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
        ZiskTranspiler::new(self.elf).runfile(
            asm_file,
            generation_method,
            log_output,
            comments,
            hints,
        )
    }

    /// Executes the conversion process, returning the Zisk ROM
    pub fn run(&self) -> Result<ZiskRom, Box<dyn Error>> {
        zisk_transpiler_common::program2rom(self.elf)
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    /// The original API still compiles, and the input still goes through the dispatcher.
    #[test]
    fn riscv2zisk_keeps_original_api() {
        let rv2zk = Riscv2zisk::new(b"not a guest program");
        assert_eq!(rv2zk.elf, b"not a guest program");
        let Err(err) = rv2zk.run() else { panic!("garbage input must be rejected") };
        assert!(err.to_string().contains("unrecognized guest format"), "{err}");
        assert!(program2rom(b"not a guest program").is_err());
    }
}
