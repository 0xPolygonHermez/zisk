//! The `KeccakfTableSM` module defines the Keccakf Table State Machine.
//!
//! This state machine is responsible for handling Keccakf operations, calculating table rows,
//! and managing multiplicity tables for Keccakf table traces.

use std::sync::{Arc, Mutex};

use p3_field::Field;
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
    multiplicity: Mutex<Vec<u64>>,
}

impl KeccakfTableSM {
    /// Creates a new `KeccakfTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `KeccakfTableSM`.
    pub fn new<F: Field>() -> Arc<Self> {
        Arc::new(Self { multiplicity: Mutex::new(vec![0; KeccakfTableTrace::<F>::NUM_ROWS]) })
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `input` - A slice of `u64` values representing the input data.
    pub fn process_slice(&self, input: &[u64]) {
        // Create the trace vector
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val;
        }
    }

    /// Detaches and returns the current multiplicity table.
    ///
    /// # Returns
    /// A vector containing the multiplicity table.
    pub fn detach_multiplicity(&self) -> Vec<u64> {
        let mut multiplicity = self.multiplicity.lock().unwrap();
        std::mem::take(&mut *multiplicity)
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
    pub fn calculate_table_row(gate_opcode: KeccakfTableGateOp, a: u64, b: u64) -> u64 {
        debug_assert!(a <= MASK_BITS);
        debug_assert!(b <= MASK_BITS);

        // Calculate the different row offset contributors, according to the PIL
        let offset_a: u64 = a;
        let offset_b: u64 = b * P2_BITS;
        let offset_opcode: u64 = Self::offset_opcode(gate_opcode);

        offset_a + offset_b + offset_opcode
    }

    /// Computes the opcode offset for the given operation.
    fn offset_opcode(gate_opcode: KeccakfTableGateOp) -> u64 {
        match gate_opcode {
            KeccakfTableGateOp::Xor => 0,
            KeccakfTableGateOp::Andp => P2_BITS_SQUARED,
        }
    }
}
