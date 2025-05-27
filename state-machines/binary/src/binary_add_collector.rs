//! The `BinaryAddCollector` struct represents an input collector for binary add operations.

use std::collections::VecDeque;

use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, OPERATION_BUS_ID,
};
use zisk_core::zisk_ops::ZiskOp;

/// The `BinaryAddCollector` struct represents an input collector for binary add operations.
pub struct BinaryAddCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<[u64; 2]>,

    pub num_operations: usize,
    pub collect_skipper: CollectSkipper,
}

impl BinaryAddCollector {
    /// Creates a new `BinaryAddCollector`.
    ///
    /// # Arguments
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - Helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `BinaryAddCollector` instance initialized with the provided parameters.
    pub fn new(num_operations: usize, collect_skipper: CollectSkipper) -> Self {
        Self { inputs: Vec::new(), num_operations, collect_skipper }
    }
}

impl BusDevice<u64> for BinaryAddCollector {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An optional vector of tuples where:
    /// - The first element is the bus ID.
    /// - The second element is always empty indicating there are no derived inputs.
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() >= self.num_operations {
            return;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op = OperationBusData::get_op(&data);

        if op != ZiskOp::Add.code() {
            return;
        }

        if self.collect_skipper.should_skip() {
            return;
        }

        self.inputs.push([OperationBusData::get_a(&data), OperationBusData::get_b(&data)]);
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
