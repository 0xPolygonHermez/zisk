//! The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use std::collections::VecDeque;

use crate::BinaryInput;
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, OPERATION_BUS_ID,
};
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
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() >= self.num_operations {
            return false;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
            return true;
        }

        if !self.with_adds && OperationBusData::get_op(&data) == ZiskOp::Add.code() {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        self.inputs.push(BinaryInput::from(&data));

        self.inputs.len() < self.num_operations
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
