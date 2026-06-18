//! The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use crate::{BinaryBasicFrops, BinaryInput};
use zisk_common::{
    BusDevice, BusId, CollectSkipper, VirtualTableSink, A, B, OP, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use std::sync::Arc;

/// The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
pub struct BinaryBasicCollector<S: VirtualTableSink> {
    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,

    pub num_operations: usize,

    pub collect_skipper: CollectSkipper,

    /// Flag to indicate that this instance comute add operations
    with_adds: bool,

    /// Flag to indicate that force to execute to end of chunk
    force_execute_to_end: bool,

    /// The table ID for the Binary FROPS
    frops_table_id: usize,

    /// Sink for virtual-table multiplicities (the real `Std` in production).
    witness: Arc<S>,
}

impl<S: VirtualTableSink> BinaryBasicCollector<S> {
    /// Creates a new `BinaryBasicCollector`.
    ///
    /// # Arguments
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - Helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `BinaryBasicCollector` instance initialized with the provided parameters.
    pub fn new(
        num_operations: usize,
        collect_skipper: CollectSkipper,
        with_adds: bool,
        force_execute_to_end: bool,
        witness: Arc<S>,
    ) -> Self {
        let frops_table_id = witness.virtual_table_id(BinaryBasicFrops::TABLE_ID);

        Self {
            inputs: Vec::with_capacity(num_operations),
            num_operations,
            collect_skipper,
            with_adds,
            force_execute_to_end,
            frops_table_id,
            witness,
        }
    }

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
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);
        // The router (`route_data`) dispatches this collector only for the
        // `Binary` op-type arm, so the fixed operation header `[op, op_type, a, b]`
        // can be read directly instead of round-tripping through `ExtOperationData`.
        debug_assert_eq!(data[OP_TYPE] as u32, ZiskOperationType::Binary as u32);

        let instance_complete = self.inputs.len() == self.num_operations;

        if instance_complete && !self.force_execute_to_end {
            return false;
        }

        let op = data[OP] as u8;
        let a = data[A];
        let b = data[B];

        if !self.with_adds && op == ZiskOp::Add.code() {
            return true;
        }

        let frops_row = BinaryBasicFrops::get_row(op, a, b);

        if self.collect_skipper.should_skip_query(frops_row == BinaryBasicFrops::NO_FROPS) {
            return true;
        }

        if frops_row != BinaryBasicFrops::NO_FROPS {
            self.witness.inc_row_one(self.frops_table_id, frops_row);
            return true;
        }

        if instance_complete {
            // instance complete => no FROPS operation => discard, inputs complete
            return true;
        }
        self.inputs.push(BinaryInput::new(op, a, b));

        self.inputs.len() < self.num_operations || self.force_execute_to_end
    }
}

impl<S: VirtualTableSink> BusDevice<u64> for BinaryBasicCollector<S> {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
