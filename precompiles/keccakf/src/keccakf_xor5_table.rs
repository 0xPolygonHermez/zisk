//! The `KeccakfXor5TableSM` module defines the Keccakf xor5 Table State Machine.
//!
//! This state machine is responsible for calculating xor5 normalization table rows.

/// The `KeccakfXor5TableSM` struct represents the Keccakf xor5 Table State Machine.
pub struct KeccakfXor5TableSM;

impl KeccakfXor5TableSM {
    /// Must mirror KECCAKF_XOR5_TABLE_ID in pil/opids.pil
    pub const TABLE_ID: usize = 127;
}
