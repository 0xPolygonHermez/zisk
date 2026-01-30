//! The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use crate::{BinaryBasicFrops, BinaryInput};
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, A, B, OP,
    OPERATION_BUS_ID,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use fields::PrimeField64;
use pil_std_lib::Std;
use std::sync::Arc;

/// The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
pub struct BinaryBasicCollector<F: PrimeField64> {
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

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryBasicCollector<F> {
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
        std: Arc<Std<F>>,
    ) -> Self {
        let frops_table_id = std
            .get_virtual_table_id(BinaryBasicFrops::TABLE_ID)
            .expect("Failed to get FROPS table ID");

        Self {
            inputs: Vec::with_capacity(num_operations),
            num_operations,
            collect_skipper,
            with_adds,
            force_execute_to_end,
            frops_table_id,
            std,
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
        let instance_complete = self.inputs.len() == self.num_operations;

        if instance_complete && !self.force_execute_to_end {
            return false;
        }

        let frops_row = BinaryBasicFrops::get_row(data[OP] as u8, data[A], data[B]);

        let op_data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&op_data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
            return true;
        }

        if !self.with_adds && OperationBusData::get_op(&op_data) == ZiskOp::Add.code() {
            return true;
        }

        if self.collect_skipper.should_skip_query(frops_row == BinaryBasicFrops::NO_FROPS) {
            return true;
        }

        if frops_row != BinaryBasicFrops::NO_FROPS {
            self.std.inc_virtual_row(self.frops_table_id, frops_row as u64, 1);
            return true;
        }

        if instance_complete {
            // instance complete => no FROPS operation => discard, inputs complete
            return true;
        }
        self.inputs.push(BinaryInput::from(&op_data));

        self.inputs.len() < self.num_operations || self.force_execute_to_end
    }
}

impl<F: PrimeField64> BusDevice<u64> for BinaryBasicCollector<F> {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
