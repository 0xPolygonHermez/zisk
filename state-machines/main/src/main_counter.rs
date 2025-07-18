//! The `MainCounter` module defines a counter for tracking publics operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::PubOut` instructions.

use std::collections::VecDeque;
use zisk_common::{BusDevice, BusId, Metrics, A, B, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::ZiskOperationType;

/// The `MainCounter` struct represents a counter that monitors and measures
/// pubOuts-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct MainCounter {
    /// Public outputs for the main state machine.
    pub publics: Vec<(u64, u32)>,
}

impl Default for MainCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl MainCounter {
    /// Creates a new instance of `MainCounter`.
    ///
    /// # Arguments
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `MainCounter` instance.
    pub fn new() -> Self {
        Self { publics: Vec::new() }
    }
}

impl Metrics for MainCounter {
    #[inline(always)]
    fn measure(&mut self, _data: &[u64]) {}

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl BusDevice<u64> for MainCounter {
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
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        const PUBOUT: u64 = ZiskOperationType::PubOut as u64;

        if data[OP_TYPE] != PUBOUT {
            return true;
        }

        let pub_index = data[A] << 1;
        let pub_value = data[B];

        self.publics.push((pub_index, (pub_value & 0xFFFFFFFF) as u32));
        self.publics.push((pub_index + 1, ((pub_value >> 32) & 0xFFFFFFFF) as u32));

        true
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
