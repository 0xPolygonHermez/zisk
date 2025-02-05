//! The `MainCounter` module defines a counter for tracking publics operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::PubOut` instructions.

use data_bus::{BusDevice, BusId, OperationBusData, OperationData};
use sm_common::Metrics;
use zisk_core::ZiskOperationType;

/// The `MainCounter` struct represents a counter that monitors and measures
/// pubOuts-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct MainCounter {
    /// The connected bus ID.
    bus_id: BusId,

    /// Public outputs for the main state machine.
    pub publics: Vec<(u64, u32)>,
}

impl MainCounter {
    /// Creates a new instance of `MainCounter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `MainCounter` instance.
    pub fn new(bus_id: BusId) -> Self {
        Self { bus_id, publics: Vec::new() }
    }
}

impl Metrics for MainCounter {
    fn measure(&mut self, _bus_id: &BusId, _data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        vec![]
    }

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
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether processing should continue.
    /// - The second element contains derived inputs to be sent back to the bus.
    #[inline]
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        let input: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&input);

        if op_type as u32 != ZiskOperationType::PubOut as u32 {
            return None;
        }

        let pub_index = 2 * OperationBusData::get_a(&input);
        let pub_value = OperationBusData::get_b(&input);

        let values = [(pub_value & 0xFFFFFFFF) as u32, ((pub_value >> 32) & 0xFFFFFFFF) as u32];

        self.publics.push((pub_index, values[0]));
        self.publics.push((pub_index + 1, values[1]));

        None
    }

    /// Returns the bus IDs associated with this counter.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
