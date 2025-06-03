//! The `ArithTableSM` module defines the Arithmetic Table State Machine.
//!
//! This state machine manages the multiplicity table for arithmetic table traces and provides
//! functionality to process inputs and manage multiplicity data.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use crate::ArithTableInputs;
use zisk_common::create_atomic_vec;
use zisk_pil::ArithTableTrace;

/// The `ArithTableSM` struct represents the Arithmetic Table State Machine.
///
/// It handles the multiplicity table for arithmetic operations and provides methods to process
/// inputs and retrieve the accumulated data.
pub struct ArithTableSM {
    /// Multiplicity table shared across threads.
    multiplicity: Vec<AtomicU64>,
    calculated: AtomicBool,
}

impl ArithTableSM {
    /// Creates a new `ArithTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithTableSM`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            multiplicity: create_atomic_vec(ArithTableTrace::<usize>::NUM_ROWS),
            calculated: AtomicBool::new(false),
        })
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `inputs` - A reference to `ArithTableInputs`, containing rows and their corresponding
    ///   values.
    pub fn process_slice(&self, inputs: &ArithTableInputs) {
        if self.calculated.load(Ordering::Relaxed) {
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
    pub fn detach_multiplicity(&self) -> &[AtomicU64] {
        &self.multiplicity
    }

    pub fn set_calculated(&self) {
        self.calculated.store(true, Ordering::Relaxed);
    }

    pub fn reset_calculated(&self) {
        self.calculated.store(false, Ordering::Relaxed);
    }
}
