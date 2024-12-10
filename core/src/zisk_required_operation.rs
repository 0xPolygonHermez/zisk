//! Data required to prove the different Zisk operations

/// Required data to make an operation.  
///
/// Stores the minimum information to reproduce an operation execution:
/// * The opcode and the a and b registers values (regardless of their sources)
/// * The step is also stored to keep track of the program execution point
///
/// This data is generated during the first emulation execution.  
/// This data is required by the main state machine executor to generate the witness computation.
#[derive(Clone)]
pub struct ZiskRequiredOperation {
    pub step: u64,
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
}

/// Stores the minimum information to generate the memory state machine witness computation.
#[derive(Clone)]
pub struct ZiskRequiredMemory {
    pub step: u64,
    pub is_write: bool,
    pub address: u64,
    pub width: u64,
    pub value: u64,
}
