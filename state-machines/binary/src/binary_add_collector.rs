//! The `BinaryAddCollector` struct represents an input collector for binary add operations.

use crate::BinaryBasicFrops;
use std::collections::VecDeque;
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, A, B, OP,
    OPERATION_BUS_ID,
};
use zisk_core::zisk_ops::ZiskOp;

/// The `BinaryAddCollector` struct represents an input collector for binary add operations.
pub struct BinaryAddCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<[u64; 2]>,
    /// Collected rows for FROPS
    pub frops_inputs: Vec<u32>,

    pub num_operations: usize,
    pub collect_skipper: CollectSkipper,

    /// Flag to indicate that force to execute to end of chunk
    force_execute_to_end: bool,
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
        Self {
            inputs: Vec::new(),
            num_operations,
            collect_skipper,
            frops_inputs: Vec::new(),
            force_execute_to_end: false,
        }
    }
}

impl BusDevice<u64> for BinaryAddCollector {
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
        let instance_complete = self.inputs.len() == self.num_operations as usize;

        if instance_complete && !self.force_execute_to_end {
            return false;
        }

        let op_data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op = OperationBusData::get_op(&op_data);

        if op != ZiskOp::Add.code() {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let frops_row = BinaryBasicFrops::get_row(data[OP] as u8, data[A], data[B]);
        if frops_row != BinaryBasicFrops::NO_FROPS {
            self.frops_inputs.push(frops_row as u32);
            return true;
        }

        if instance_complete {
            // instance complete => no FROPS operation => discard, inputs complete
            return true;
        }

        self.inputs.push([OperationBusData::get_a(&op_data), OperationBusData::get_b(&op_data)]);

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
