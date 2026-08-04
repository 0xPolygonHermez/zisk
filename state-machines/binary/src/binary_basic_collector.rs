//! The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use crate::{
    add_shape, BinaryBasicFrops, BinaryCollectCursor, BinaryCollectInfo, BinaryInput, CollectAction,
};
use zisk_common::{
    BusDevice, BusId, ExtOperationData, OperationBusData, A, B, OP, OPERATION_BUS_ID,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use fields::PrimeField64;
use pil_std_lib::Std;
use std::sync::Arc;

/// The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
pub struct BinaryBasicCollector<F: PrimeField64> {
    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,

    /// Decides, operation by operation, what belongs to this instance.
    cursor: BinaryCollectCursor,

    /// The table ID for the Binary FROPS
    frops_table_id: usize,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryBasicCollector<F> {
    /// Creates a new `BinaryBasicCollector`.
    ///
    /// # Arguments
    /// * `collect_info` - What this instance collects from the chunk: how many operations, how many
    ///   to skip, and which additions belong to a dedicated air.
    /// * `std` - PIL2 standard library utilities.
    ///
    /// # Returns
    /// A new `BinaryBasicCollector` instance initialized with the provided parameters.
    pub fn new(collect_info: BinaryCollectInfo, std: Arc<Std<F>>) -> Self {
        let frops_table_id = std
            .get_virtual_table_id(BinaryBasicFrops::TABLE_ID)
            .expect("Failed to get FROPS table ID");

        Self {
            inputs: Vec::with_capacity(collect_info.count as usize),
            cursor: BinaryCollectCursor::new(collect_info),
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

        let frops_row = BinaryBasicFrops::get_row(data[OP] as u8, data[A], data[B]);

        let op_data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        if OperationBusData::get_op_type(&op_data) as u32 != ZiskOperationType::Binary as u32 {
            return true;
        }

        // Additions are bucketed by shape, since the planner routes each shape independently.
        let shape = (OperationBusData::get_op(&op_data) == ZiskOp::Add.code())
            .then(|| add_shape(data[A], data[B]));

        match self.cursor.next(shape, frops_row != BinaryBasicFrops::NO_FROPS) {
            CollectAction::Stop => false,
            CollectAction::Pass => true,
            CollectAction::CountFrop => {
                self.std.inc_virtual_row_one(self.frops_table_id, frops_row);
                true
            }
            CollectAction::Collect => {
                self.inputs.push(BinaryInput::from(&op_data));
                !self.cursor.is_done()
            }
        }
    }
}

impl<F: PrimeField64> BusDevice<u64> for BinaryBasicCollector<F> {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
