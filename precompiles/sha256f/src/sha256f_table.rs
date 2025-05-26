//! The `Sha256fTableSM` module defines the Sha256f Table State Machine.
//!
//! This state machine is responsible for handling Sha256f operations, calculating table rows,
//! and managing multiplicity tables for Sha256f table traces.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use p3_field::Field;
use proofman_common::PaddedAtomicU64;
use zisk_common::create_atomic_vec;
use zisk_pil::Sha256fTableTrace;

use crate::sha256f_constants::*;

/// Represents operations supported by the Sha256f Table.
#[repr(u8)]
pub enum Sha256fTableGateOp {
    /// XOR gate
    Xor = XOR_GATE_OP,

    /// CH gate
    Ch = CH_GATE_OP,

    /// MAJ gate
    Maj = MAJ_GATE_OP,

    /// ADD gate
    Add = ADD_GATE_OP,
}

/// The `Sha256fTableSM` struct represents the Sha256f Table State Machine.
///
/// It manages a multiplicity table and provides functionality to process slices and calculate table
/// rows.
pub struct Sha256fTableSM {
    /// The multiplicity table, shared across threads.
    multiplicities: Vec<Vec<PaddedAtomicU64>>,
    calculated: AtomicBool,
}

impl Sha256fTableSM {
    /// Creates a new `Sha256fTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `Sha256fTableSM`.
    pub fn new<F: Field>() -> Arc<Self> {
        let mut multiplicities = Vec::new();
        for _ in 0..Sha256fTableTrace::<usize>::ROW_SIZE {
            multiplicities.push(create_atomic_vec(Sha256fTableTrace::<usize>::NUM_ROWS));
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

    /// Detaches and returns the current multiplicity table.
    ///
    /// # Returns
    /// A vector containing the multiplicity table.
    pub fn detach_multiplicities(&self) -> &[Vec<PaddedAtomicU64>] {
        &self.multiplicities
    }

    pub fn set_calculated(&self) {
        self.calculated.store(true, Ordering::Relaxed);
    }

    pub fn reset_calculated(&self) {
        self.calculated.store(false, Ordering::SeqCst);
    }

    /// Calculates the table row offset based on the provided parameters.
    ///
    /// # Arguments
    /// * `gate_opcode` - The operation code (`Sha256fTableGateOp`).
    /// * `a` - The first operand a.
    /// * `b` - The second operand b.
    ///
    /// # Returns
    /// The calculated table row offset.
    pub fn calculate_table_row(gate_opcode: &Sha256fTableGateOp, a: u64, b: u64, c: u64) -> usize {
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
    fn offset_opcode(gate_opcode: &Sha256fTableGateOp) -> u64 {
        match gate_opcode {
            Sha256fTableGateOp::Xor => 0,
            Sha256fTableGateOp::Ch => P2_BITS_ABC,
            Sha256fTableGateOp::Maj => 2 * P2_BITS_ABC,
            Sha256fTableGateOp::Add => 3 * P2_BITS_ABC,
        }
    }

    pub fn acc_local_multiplicity(&self, local_sha256f_table_sm: &Sha256fTableSM) {
        if self.calculated.load(Ordering::SeqCst) {
            return;
        }
        // TODO: PARALLEL ???
        for (i, multiplicity) in local_sha256f_table_sm.multiplicities[0].iter().enumerate() {
            let value = multiplicity.load(Ordering::Relaxed);
            if value != 0 {
                self.multiplicities[0][i].fetch_add(value, Ordering::Relaxed);
            }
        }
    }
}
