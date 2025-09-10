//! Error types for compressed instruction decoding
use thiserror::Error;

/// Errors that can occur during compressed instruction decoding
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    #[error("Invalid compressed instruction")]
    InvalidInstruction,

    #[error("Reserved compressed instruction encoding")]
    Reserved,

    #[error("Instruction not supported on target configuration")]
    UnsupportedOnTarget,

    #[error("Opcode not supported by target. Opcode field is {opcode_bits}")]
    UnsupportedOpcode { opcode_bits: u8 },
}
