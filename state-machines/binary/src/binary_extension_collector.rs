//! The `BinaryExtensionCollector` struct represents an input collector for binary extension
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use crate::{
    extension_requires_full, BinaryCollectCursor, BinaryExtensionFrops, BinaryInput, ChunkCollect,
    CollectAction, EXT_KINDS, KIND_EXT_CLEAN, KIND_EXT_DIRTY,
};
use zisk_common::{
    BusDevice, BusId, ExtOperationData, OperationBusData, A, B, OP, OPERATION_BUS_ID,
};

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use std::sync::Arc;

use zisk_core::ZiskOperationType;

/// The `BinaryExtensionCollector` struct represents an input collector for binary extension
pub struct BinaryExtensionCollector<F: PrimeField64> {
    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,

    /// Decides, operation by operation, what belongs to this instance.
    cursor: BinaryCollectCursor<EXT_KINDS>,

    /// The table ID for the Binary Extension FROPS
    frops_table_id: usize,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryExtensionCollector<F> {
    /// Creates a new `BinaryExtensionCollector`.
    ///
    /// # Arguments
    /// * `collect` - What this instance takes from the chunk: a `(count, skip)` per kind of
    ///   operation, plus which of the chunk's frequent operations it accounts for.
    /// * `std` - PIL2 standard library utilities.
    ///
    /// # Returns
    /// A new `BinaryExtensionCollector` ready to replay the chunk.
    pub fn new(collect: ChunkCollect<EXT_KINDS>, std: Arc<Std<F>>) -> Self {
        let frops_table_id = std
            .get_virtual_table_id(BinaryExtensionFrops::TABLE_ID)
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

        if OperationBusData::get_op_type(&op_data) as u32 != ZiskOperationType::BinaryE as u32 {
            return true;
        }

        // Operations whose unused operand parts are dirty can only be proven by the full air.
        let kind = if extension_requires_full(data[OP] as u8, data[A], data[B]) {
            KIND_EXT_DIRTY
        } else {
            KIND_EXT_CLEAN
        };

        let frops_row = BinaryExtensionFrops::get_row(data[OP] as u8, data[A], data[B]);

        match self.cursor.next(kind, frops_row != BinaryExtensionFrops::NO_FROPS) {
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

impl<F: PrimeField64> BusDevice<u64> for BinaryExtensionCollector<F> {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
