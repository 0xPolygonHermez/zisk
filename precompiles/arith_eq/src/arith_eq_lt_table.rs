//! The `ArithEqLtTableSM` module defines the ArithEqLt Table State Machine.
//!
//! This state machine is responsible for handling verification of chunks are less than
//! module or prime chunk

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use zisk_common::create_atomic_vec;
use zisk_pil::ArithEqLtTableTrace;

/// The `ArithEqLtTableSM` struct represents the ArithEqLt Table State Machine.
///
/// It manages a multiplicity table and provides functionality to process slices and calculate table
/// rows.
pub struct ArithEqLtTableSM {
    /// The multiplicity table, shared across threads.
    multiplicities: Vec<Vec<AtomicU64>>,
    calculated: AtomicBool,
}

impl ArithEqLtTableSM {
    /// Creates a new `ArithEqLtTableSM` instance.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithEqLtTableSM`.
    pub fn new() -> Arc<Self> {
        let mut multiplicities = Vec::new();
        for _ in 0..ArithEqLtTableTrace::<usize>::ROW_SIZE {
            multiplicities.push(create_atomic_vec(ArithEqLtTableTrace::<usize>::NUM_ROWS));
        }
        Arc::new(Self { multiplicities, calculated: AtomicBool::new(false) })
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
    /// * `prev_lt` - If previous current chunk of a is less than b, false at beginning
    /// * `lt` - If current chunk of a is less than b
    /// * `delta` - Difference between to values to compare (a - b)
    ///
    /// # Returns
    /// The calculated table row offset.
    pub fn update_input(&self, prev_lt: bool, lt: bool, delta: i64) {
        if self.calculated.load(Ordering::Relaxed) {
            return;
        }

        // 0 0..MAX16
        // 1 -1..MIN16
        // 2 1..MAX16
        // 3 0..MIN16

        let index = match 2 * prev_lt as u8 + lt as u8 {
            0 => delta as usize,
            1 => 0x10000 + (-delta - 1) as usize,
            2 => 0x20000 + (delta - 1) as usize,
            3 => 0x30000 + (-delta) as usize,
            _ => panic!("Invalid range type"),
        };
        if index > 0x3FFFF {
            panic!("Invalid index:{index} prev_lt:{prev_lt} lt:{lt} delta:{delta}");
        }
        self.multiplicities[0][index].fetch_add(1, Ordering::Relaxed);
    }
}
