//! The `FrequentOpsCounter` module defines a counter for tracking all frequents operations
//! sent over the data bus.

use std::{any::Any, collections::VecDeque};

use zisk_common::{BusDevice, BusId, Metrics, OPERATION_BUS_ID};

/// The `FrequentOpsCounter` struct represents a counter that monitors frequent operations
/// on the data bus.
///
#[derive(Default)]
pub struct FrequentOpsCounter {
    counter: u64,
}

impl FrequentOpsCounter {
    /// Phantom data to allow generic usage with different types.
    pub fn new() -> Self {
        Self::default()
    }
}
impl Metrics for FrequentOpsCounter {
    /// Empty API method implementation for measuring metrics because the counters are not used,
    /// always exists one and only one instance
    #[inline(always)]
    fn measure(&mut self, _data: &[u64]) {}

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BusDevice<u64> for FrequentOpsCounter {
    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline]
    fn process_data(
        &mut self,
        _bus_id: &BusId,
        _data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        // Always report that frequent operations exist, because at least one jump operation uses them.
        if self.counter == 0 {
            self.counter += 1;
            true
        } else {
            false
        }
    }
    /// Returns the bus IDs associated with this counter.
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
