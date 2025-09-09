/// Error denotes errors that can occur while using the RISCV
/// 32-bit decoder.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unsupported extension: {0}")]
    UnsupportedExtension(String),

    #[error("Invalid instruction format")]
    InvalidFormat,

    #[error("Instruction not supported by target")]
    UnsupportedInstruction,
}
