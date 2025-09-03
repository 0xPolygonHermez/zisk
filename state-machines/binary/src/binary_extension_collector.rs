//! The `BinaryExtensionCollector` struct represents an input collector for binary extension
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use std::collections::VecDeque;

use crate::{BinaryExtensionFrops, BinaryInput};
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, A, B, OP,
    OPERATION_BUS_ID,
};
use zisk_core::ZiskOperationType;

/// The `BinaryExtensionCollector` struct represents an input collector for binary extension
pub struct BinaryExtensionCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,
    /// Collected rows for FROPS
    pub frops_inputs: Vec<u32>,

    pub num_operations: usize,
    pub collect_skipper: CollectSkipper,

    /// Flag to indicate that force to execute to end of chunk
    force_execute_to_end: bool,
}

impl BinaryExtensionCollector {
    pub fn new(
        num_operations: usize,
        num_freq_ops: usize,
        collect_skipper: CollectSkipper,
        force_execute_to_end: bool,
    ) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations),
            num_operations,
            collect_skipper,
            frops_inputs: Vec::with_capacity(num_freq_ops),
            force_execute_to_end,
        }
    }
}

impl BusDevice<u64> for BinaryExtensionCollector {
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
        let instance_complete = self.inputs.len() == self.num_operations;

        if instance_complete && !self.force_execute_to_end {
            return false;
        }

        let frops_row = BinaryExtensionFrops::get_row(data[OP] as u8, data[A], data[B]);

        let op_data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&op_data);

        if op_type as u32 != ZiskOperationType::BinaryE as u32 {
            return true;
        }

        if self.collect_skipper.should_skip_query(frops_row == BinaryExtensionFrops::NO_FROPS) {
            return true;
        }

        if frops_row != BinaryExtensionFrops::NO_FROPS {
            self.frops_inputs.push(frops_row as u32);
            return true;
        }

        if instance_complete {
            // instance complete => no FROPS operation => discard, inputs complete
            return true;
        }

        self.inputs.push(BinaryInput::from(&op_data));

        self.inputs.len() < self.num_operations || self.force_execute_to_end
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
