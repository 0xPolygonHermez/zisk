//! The `ArithTableSM` module defines the Arithmetic Table State Machine.
//!
//! This state machine manages the multiplicity table for arithmetic table traces and provides
//! functionality to process inputs and manage multiplicity data.

use std::sync::{Arc, Mutex};

use crate::ArithTableInputs;
use zisk_pil::ArithTableTrace;

/// The `ArithTableSM` struct represents the Arithmetic Table State Machine.
///
/// It handles the multiplicity table for arithmetic operations and provides methods to process
/// inputs and retrieve the accumulated data.
pub struct ArithTableSM {
    /// Multiplicity table shared across threads.
    multiplicity: Mutex<Vec<u64>>,
    used: AtomicBool,
}

impl ArithTableSM {
    /// Creates a new `ArithTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithTableSM`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { multiplicity: Mutex::new(vec![0; ArithTableTrace::<usize>::NUM_ROWS]) })
    }

    /// Processes a slice of input data and updates the multiplicity table.
    ///
    /// # Arguments
    /// * `inputs` - A reference to `ArithTableInputs`, containing rows and their corresponding
    ///   values.
    pub fn process_slice(&self, inputs: &ArithTableInputs) {
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
