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

    #[error("Not a compressed instruction (bits [1:0] = 11)")]
    NotCompressed,
}
