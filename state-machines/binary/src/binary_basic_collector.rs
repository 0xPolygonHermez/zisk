//! The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use crate::BinaryInput;
use sm_common::CollectSkipper;
use zisk_common::{BusDevice, BusId, ExtOperationData, OperationBusData, OPERATION_BUS_ID};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

/// The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
pub struct BinaryBasicCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,

    pub num_operations: usize,
    pub collect_skipper: CollectSkipper,

    /// Flag to indicate that this instance comute add operations
    with_adds: bool,
}

impl BinaryBasicCollector {
    /// Creates a new `BinaryBasicCollector`.
    ///
    /// # Arguments
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - Helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `BinaryBasicCollector` instance initialized with the provided parameters.
    pub fn new(num_operations: usize, collect_skipper: CollectSkipper, with_adds: bool) -> Self {
        Self { inputs: Vec::new(), num_operations, collect_skipper, with_adds }
    }
}

impl BusDevice<u64> for BinaryBasicCollector {
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
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() >= self.num_operations {
            return None;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
            return None;
        }

        if !self.with_adds && OperationBusData::get_op(&data) == ZiskOp::Add.code() {
            return None;
        }

        if self.collect_skipper.should_skip() {
            return None;
        }

        self.inputs.push(BinaryInput::from(&data));
        None
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
