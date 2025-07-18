//! The `KeccakfTableSM` module defines the Keccakf Table State Machine.
//!
//! This state machine is responsible for handling Keccakf operations, calculating table rows,
//! and managing multiplicity tables for Keccakf table traces.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use fields::Field;
use zisk_common::create_atomic_vec;
use zisk_pil::KeccakfTableTrace;

use crate::{
    MASK_BITS_A, MASK_BITS_B, MASK_BITS_C, P2_BITS_A, P2_BITS_AB, P2_BITS_ABC, XOR_ANDP_GATE_OP,
    XOR_GATE_OP,
};

/// Represents operations supported by the Keccakf Table.
#[repr(u8)]
pub enum KeccakfTableGateOp {
    /// XOR gate
    Xor = XOR_GATE_OP,

    /// XORANDP gate
    XorAndp = XOR_ANDP_GATE_OP,
}

/// The `KeccakfTableSM` struct represents the Keccakf Table State Machine.
///
/// It manages a multiplicity table and provides functionality to process slices and calculate table
/// rows.
pub struct KeccakfTableSM {
    /// The multiplicity table, shared across threads.
    multiplicities: Vec<Vec<AtomicU64>>,
    calculated: AtomicBool,
}

impl KeccakfTableSM {
    /// Creates a new `KeccakfTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `KeccakfTableSM`.
    pub fn new<F: Field>() -> Arc<Self> {
        let mut multiplicities = Vec::new();
        for _ in 0..KeccakfTableTrace::<usize>::ROW_SIZE {
            multiplicities.push(create_atomic_vec(KeccakfTableTrace::<usize>::NUM_ROWS));
        }
        Arc::new(Self { multiplicities, calculated: AtomicBool::new(false) })
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `input` - A slice of `u64` values representing the input data.
    pub fn update_input(&self, index: usize, value: u64) {
        if self.calculated.load(Ordering::Relaxed) {
            return;
        }
        self.multiplicities[0][index].fetch_add(value, Ordering::Relaxed);
    }

    pub fn update_multiplicities(&self, multiplicities: &[u64]) {
        if self.calculated.load(Ordering::Relaxed) {
            return;
        }
        for (index, &value) in multiplicities.iter().enumerate() {
            if value != 0 {
                self.multiplicities[0][index].fetch_add(value, Ordering::Relaxed);
            }
        }
    }

    /// Detaches and returns the current multiplicity table.
    ///
    /// # Returns
    /// A vector containing the multiplicity table.
    pub fn detach_multiplicities(&self) -> &[Vec<AtomicU64>] {
        &self.multiplicities
    }

    pub fn set_calculated(&self) {
        self.calculated.store(true, Ordering::Relaxed);
    }

    pub fn reset_calculated(&self) {
        self.calculated.store(false, Ordering::Relaxed);
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
    pub fn calculate_table_row(gate_opcode: &KeccakfTableGateOp, a: u64, b: u64, c: u64) -> usize {
        debug_assert!(a <= MASK_BITS_A);
        debug_assert!(b <= MASK_BITS_B);
        debug_assert!(c <= MASK_BITS_C);

        // Calculate the different row offset contributors, according to the PIL
        let offset_a: u64 = a;
        let offset_b: u64 = b * P2_BITS_A;
        let offset_c: u64 = c * P2_BITS_AB;
        let offset_opcode: u64 = Self::offset_opcode(gate_opcode);

        (offset_a + offset_b + offset_c + offset_opcode)
            .try_into()
            .expect("Invalid table row offset")
    }

    /// Computes the opcode offset for the given operation.
    fn offset_opcode(gate_opcode: &KeccakfTableGateOp) -> u64 {
        match gate_opcode {
            KeccakfTableGateOp::Xor => 0,
            KeccakfTableGateOp::XorAndp => P2_BITS_ABC,
        }
    }
}
