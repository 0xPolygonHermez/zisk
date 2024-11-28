//! Data required to prove the different Zisk operations

use std::collections::HashMap;

/// Stores the minimum information to reproduce an operation execution:
/// the opcode and the a and b registers values (regardless of their sources);
/// the step is also stored to keep track of the program execution point.
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

/// Data required to get some operations proven by the secondary state machine
#[derive(Clone, Default)]
pub struct ZiskRequired {
    pub arith: Vec<ZiskRequiredOperation>,
    pub binary: Vec<ZiskRequiredOperation>,
    pub binary_extension: Vec<ZiskRequiredOperation>,
    pub memory: Vec<ZiskRequiredMemory>,
}

/// Histogram of the program counter values used during the program execution.
/// Each pc value has a u64 counter, associated to it via a hash map.
/// The counter is increased every time the corresponding instruction is executed.
#[derive(Clone, Default)]
pub struct ZiskPcHistogram {
    pub map: HashMap<u64, u64>,
    pub end_pc: u64,
    pub steps: u64,
}
