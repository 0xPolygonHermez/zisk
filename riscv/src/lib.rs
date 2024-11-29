//! RISC-V instruction structure and parser.  
//! The riscv_interpreter function accepts a buffer of bytes (a slice of u8), parses it according to
//! the RISC-V spec, and generates a vector of RiscvInstruction's

pub mod riscv_inst;
pub mod riscv_interpreter;
pub mod riscv_registers;
pub mod riscv_rvd;

pub use riscv_inst::*;
pub use riscv_interpreter::*;
pub use riscv_registers::*;
pub use riscv_rvd::*;
