//! Entry point of the ZisK transpilers: detects the guest program's format and calls the
//! matching transpiler (`zisk-riscv` for RISC-V ELF, `zisk-transpiler-wasm` for WebAssembly).
//! See `transpilers/README.md`.

pub mod transpiler;

pub use transpiler::*;
