//! The `ArithRangeTableSM` module defines the Arithmetic Range Table State Machine.
//!
//! This state machine manages the multiplicity table for arithmetic range table traces
//! and provides functionality to process inputs and manage multiplicity data.

use std::sync::{Arc, Mutex};

use crate::ArithRangeTableInputs;
use zisk_pil::ArithRangeTableTrace;

/// The `ArithRangeTableSM` struct represents the Arithmetic Range Table State Machine.
///
/// It handles the multiplicity table for arithmetic range table operations and provides
/// methods to process inputs and retrieve the accumulated data.
pub struct ArithRangeTableSM {
    /// Multiplicity table shared across threads.
    multiplicity: Mutex<Vec<u64>>,
}

impl ArithRangeTableSM {
    /// Creates a new `ArithRangeTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithRangeTableSM`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            multiplicity: Mutex::new(vec![0; ArithRangeTableTrace::<usize>::NUM_ROWS]),
        })
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `inputs` - A reference to `ArithRangeTableInputs`, containing rows and their corresponding
    ///   values.
    pub fn process_slice(&self, inputs: &ArithRangeTableInputs) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (row, value) in inputs {
            multiplicity[row] += value;
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
}
