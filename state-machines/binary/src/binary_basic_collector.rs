//! The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use crate::{
    add_shape, AddShape, BinaryBasicFrops, BinaryCollectCursor, BinaryInput, ChunkCollect,
    CollectAction, ADD_KINDS, KIND_ADD_FULL, KIND_ADD_HI, KIND_BASIC,
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
    cursor: BinaryCollectCursor<ADD_KINDS>,

    /// The table ID for the Binary FROPS
    frops_table_id: usize,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryBasicCollector<F> {
    /// Creates a new `BinaryBasicCollector`.
    ///
    /// # Arguments
    /// * `collect` - What this instance takes from the chunk: a `(count, skip)` per kind of
    ///   operation, plus which of the chunk's frequent operations it accounts for.
    /// * `std` - PIL2 standard library utilities.
    ///
    /// # Returns
    /// A new `BinaryBasicCollector` ready to replay the chunk.
    pub fn new(collect: ChunkCollect<ADD_KINDS>, std: Arc<Std<F>>) -> Self {
        let frops_table_id = std
            .get_virtual_table_id(BinaryBasicFrops::TABLE_ID)
            .expect("Failed to get FROPS table ID");

        Self { inputs: Vec::new(), cursor: BinaryCollectCursor::new(collect), frops_table_id, std }
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

        let op_data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        // Cheapest test first: every operation on the bus reaches here, and most are not this air's.
        if OperationBusData::get_op_type(&op_data) as u32 != ZiskOperationType::Binary as u32 {
            return true;
        }

        // Additions are split by operand shape, since the planner places each shape independently.
        let kind = if OperationBusData::get_op(&op_data) == ZiskOp::Add.code() {
            match add_shape(data[A], data[B]) {
                AddShape::Hi | AddShape::HiNeg => KIND_ADD_HI,
                AddShape::Full => KIND_ADD_FULL,
            }
        } else {
            KIND_BASIC
        };

        let frops_row = BinaryBasicFrops::get_row(data[OP] as u8, data[A], data[B]);

        match self.cursor.next(kind, frops_row != BinaryBasicFrops::NO_FROPS) {
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
