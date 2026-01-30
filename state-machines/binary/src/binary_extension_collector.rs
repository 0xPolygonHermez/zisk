//! The `BinaryExtensionCollector` struct represents an input collector for binary extension
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use crate::{BinaryExtensionFrops, BinaryInput};
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, A, B, OP,
    OPERATION_BUS_ID,
};

use fields::PrimeField64;
use pil_std_lib::Std;
use std::sync::Arc;

use zisk_core::ZiskOperationType;

/// The `BinaryExtensionCollector` struct represents an input collector for binary extension
pub struct BinaryExtensionCollector<F: PrimeField64> {
    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,

    pub num_operations: usize,
    pub collect_skipper: CollectSkipper,

    /// Flag to indicate that force to execute to end of chunk
    force_execute_to_end: bool,

    /// The table ID for the Binary Extension FROPS
    frops_table_id: usize,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryExtensionCollector<F> {
    pub fn new(
        num_operations: usize,
        collect_skipper: CollectSkipper,
        force_execute_to_end: bool,
        std: Arc<Std<F>>,
    ) -> Self {
        let frops_table_id = std
            .get_virtual_table_id(BinaryExtensionFrops::TABLE_ID)
            .expect("Failed to get FROPS table ID");
        Self {
            inputs: Vec::with_capacity(num_operations),
            num_operations,
            collect_skipper,
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

impl<F: PrimeField64> BusDevice<u64> for BinaryExtensionCollector<F> {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
