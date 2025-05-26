//! The `ArithRangeTableSM` module defines the Arithmetic Range Table State Machine.
//!
//! This state machine manages the multiplicity table for arithmetic range table traces
//! and provides functionality to process inputs and manage multiplicity data.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::ArithRangeTableInputs;
use proofman_common::PaddedAtomicU64;
use zisk_common::create_atomic_vec;
use zisk_pil::ArithRangeTableTrace;

/// The `ArithRangeTableSM` struct represents the Arithmetic Range Table State Machine.
///
/// It handles the multiplicity table for arithmetic range table operations and provides
/// methods to process inputs and retrieve the accumulated data.
pub struct ArithRangeTableSM {
    /// Multiplicity table shared across threads.
    multiplicity: Vec<PaddedAtomicU64>,
    calculated: AtomicBool,
}

impl ArithRangeTableSM {
    /// Creates a new `ArithRangeTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithRangeTableSM`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            multiplicity: create_atomic_vec(ArithRangeTableTrace::<usize>::NUM_ROWS),
            calculated: AtomicBool::new(false),
        })
    }

    pub fn process_slice(&self, inputs: &ArithRangeTableInputs) {
        if self.calculated.load(Ordering::SeqCst) {
            return;
        }
        for (row, value) in inputs {
            self.multiplicity[row].fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Detaches and returns the current multiplicity table.
    ///
    /// # Returns
    /// A vector containing the multiplicity table.
    pub fn detach_multiplicity(&self) -> &[PaddedAtomicU64] {
        &self.multiplicity
    }

    pub fn set_calculated(&self) {
        self.calculated.store(true, Ordering::SeqCst);
    }

    pub fn reset_calculated(&self) {
        self.calculated.store(false, Ordering::SeqCst);
    }

    pub fn acc_local_multiplicity(&self, local_arith_range_table_sm: &ArithRangeTableSM) {
        if self.calculated.load(Ordering::SeqCst) {
            return;
        }
        // TODO: PARALLEL ???
        for (i, multiplicity) in local_arith_range_table_sm.multiplicity.iter().enumerate() {
            let value = multiplicity.load(Ordering::Relaxed);
            if value != 0 {
                self.multiplicity[i].fetch_add(value, Ordering::Relaxed);
            }
        }
    }
}
