//! Guest-program-format dispatcher.
//!
//! ZisK accepts more than one guest machine.  The transpilation entry point inspects the magic
//! bytes of the input program and routes it to the right frontend:
//!
//! * RISC-V ELF (`\x7fELF`) -> [`elf2rom`], the original bare-metal RISC-V transpiler.
//! * WebAssembly (`\0asm`)  -> [`crate::wasm::wasm2rom`], the wasm32-wasi frontend.
//!
//! Both frontends produce an architecture-neutral [`ZiskRom`], so everything downstream (emulator,
//! state machines, prover) is unaffected by which guest machine produced it.

use crate::{elf2rom, is_elf_file, is_wasm_file, wasm, ZiskRom};
use std::error::Error;

/// Transpiles a guest program (RISC-V ELF or WebAssembly) into a [`ZiskRom`].
///
/// The guest machine is detected from the program's magic bytes.
pub fn program2rom(program: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    if is_elf_file(program).unwrap_or(false) {
        elf2rom(program)
    } else if is_wasm_file(program) {
        wasm::wasm2rom(program)
    } else {
        Err("Unrecognized guest program format: expected a RISC-V ELF or a WebAssembly binary"
            .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{is_elf_file, is_wasm_file};

    #[test]
    fn detects_wasm_magic() {
        let wasm = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        assert!(is_wasm_file(&wasm));
        assert!(!is_elf_file(&wasm).unwrap());
    }

    #[test]
    fn unrecognized_format_is_rejected() {
        let garbage = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33];
        let err = program2rom(&garbage).unwrap_err().to_string();
        assert!(err.contains("Unrecognized"), "unexpected error: {err}");
    }
}
