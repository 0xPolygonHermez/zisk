//! The `FrequentOpsCollector` struct represents an input collector for frequent operations.
//!
//! It manages collected inputs for the `FrequentOpsSM` to compute witnesses

use std::collections::VecDeque;

use crate::FrequentOpsTable;
use zisk_common::{BusDevice, BusId, A, B, OP, OPERATION_BUS_ID};

/// The `FrequentOpsCollector` struct represents an input collector for frequent operations.
#[derive(Default)]
pub struct FrequentOpsCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<u32>,
}

impl FrequentOpsCollector {
    /// Creates a new `FrequentOpsCollector`.
    ///
    /// # Returns
    /// A new `FrequentOpsCollector` instance initialized with an empty inputs vector.
    pub fn new() -> Self {
        Self::default()
    }
}

impl BusDevice<u64> for FrequentOpsCollector {
    /// Processes data received on the bus, collecting inputs for frequent operations witness computation.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus containing operation, operand A, and operand B.
    /// * `_pending` – A queue of pending bus operations (unused in this implementation).
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution.
    /// Always returns `true` to continue execution.
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);
        if let Some(row) = FrequentOpsTable::get_row(data[OP] as u8, data[A], data[B]) {
            self.inputs.push(row as u32);
        }
        true
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
