//! The `ArithRangeTableSM` module defines the Arithmetic Range Table State Machine.
//!
//! This state machine manages the multiplicity table for arithmetic range table traces
//! and provides functionality to process inputs and manage multiplicity data.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::ArithRangeTableInputs;
use sm_common::create_atomic_vec;
use zisk_pil::ArithRangeTableTrace;

/// The `ArithRangeTableSM` struct represents the Arithmetic Range Table State Machine.
///
/// It handles the multiplicity table for arithmetic range table operations and provides
/// methods to process inputs and retrieve the accumulated data.
pub struct ArithRangeTableSM {
    /// Multiplicity table shared across threads.
    multiplicity: Vec<AtomicU64>,
}

impl ArithRangeTableSM {
    /// Creates a new `ArithRangeTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithRangeTableSM`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { multiplicity: create_atomic_vec(ArithRangeTableTrace::<usize>::NUM_ROWS) })
    }

    pub fn process_slice(&self, inputs: &ArithRangeTableInputs) {
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
}
