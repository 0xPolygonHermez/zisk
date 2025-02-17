//! The `KeccakfTableSM` module defines the Keccakf Table State Machine.
//!
//! This state machine is responsible for handling Keccakf operations, calculating table rows,
//! and managing multiplicity tables for Keccakf table traces.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use p3_field::Field;
use sm_common::create_atomic_vec;
use zisk_pil::KeccakfTableTrace;

use crate::keccakf_constants::*;

/// Represents operations supported by the Keccakf Table.
#[repr(u8)]
pub enum KeccakfTableGateOp {
    /// XOR gate
    Xor = XOR_GATE_OP,

    /// ANDP gate
    Andp = ANDP_GATE_OP,
}

/// The `KeccakfTableSM` struct represents the Keccakf Table State Machine.
///
/// It manages a multiplicity table and provides functionality to process slices and calculate table
/// rows.
pub struct KeccakfTableSM {
    /// The multiplicity table, shared across threads.
    multiplicity: Vec<AtomicU64>,
}

impl KeccakfTableSM {
    /// Creates a new `KeccakfTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `KeccakfTableSM`.
    pub fn new<F: Field>() -> Arc<Self> {
        Arc::new(Self { multiplicity: create_atomic_vec(KeccakfTableTrace::<usize>::NUM_ROWS) })
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `input` - A slice of `u64` values representing the input data.
    pub fn process_slice(&self, input: &[u64]) {
        for (i, val) in input.iter().enumerate() {
            self.multiplicity[i].fetch_add(*val, Ordering::Relaxed);
        }
    }

    /// Detaches and returns the current multiplicity table.
    ///
    /// # Returns
    /// A vector containing the multiplicity table.
    pub fn detach_multiplicity(&self) -> &[AtomicU64] {
        &self.multiplicity
    }

    /// Calculates the table row offset based on the provided parameters.
    ///
    /// # Arguments
    /// * `gate_opcode` - The operation code (`KeccakfTableGateOp`).
    /// * `a` - The first operand a.
    /// * `b` - The second operand b.
    ///
    /// # Returns
    /// The calculated table row offset.
    pub fn calculate_table_row(gate_opcode: &KeccakfTableGateOp, a: u64, b: u64) -> usize {
        debug_assert!(a <= MASK_BITS_A);
        debug_assert!(b <= MASK_BITS_B);

        // Calculate the different row offset contributors, according to the PIL
        let offset_a: u64 = a;
        let offset_b: u64 = b * P2_BITS_A;
        let offset_opcode: u64 = Self::offset_opcode(gate_opcode);

        (offset_a + offset_b + offset_opcode).try_into().expect("Invalid table row offset")
    }

    /// Computes the opcode offset for the given operation.
    fn offset_opcode(gate_opcode: &KeccakfTableGateOp) -> u64 {
        match gate_opcode {
            KeccakfTableGateOp::Xor => 0,
            KeccakfTableGateOp::Andp => P2_BITS_AB,
        }
    }
}
