//! The `BinaryAddHiCollector` struct represents an input collector for the packed add operations
//! proven by `BinaryAddHi`.

use crate::{
    add_shape, AddShape, BinaryBasicFrops, BinaryCollectCursor, ChunkCollect, CollectAction,
    ADD_KINDS, KIND_ADD_FULL, KIND_ADD_HI,
};
use zisk_common::{BusDevice, BusId, ExtOperationData, OperationBusData, A, B, OPERATION_BUS_ID};
use zisk_core::zisk_ops::ZiskOp;

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use std::sync::Arc;

/// The `BinaryAddHiCollector` struct represents an input collector for packed add operations.
pub struct BinaryAddHiCollector<F: PrimeField64> {
    /// Collected inputs for witness computation.
    pub inputs: Vec<[u64; 2]>,

    /// Decides, operation by operation, what belongs to this instance.
    cursor: BinaryCollectCursor<ADD_KINDS>,

    /// The table ID for the Binary Add FROPS
    frops_table_id: usize,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryAddHiCollector<F> {
    /// Creates a new `BinaryAddHiCollector`.
    ///
    /// # Arguments
    /// * `collect` - What this instance takes from the chunk: a `(count, skip)` per kind of
    ///   operation, plus which of the chunk's frequent operations it accounts for.
    /// * `std` - PIL2 standard library utilities.
    ///
    /// # Returns
    /// A new `BinaryAddHiCollector` ready to replay the chunk.
    pub fn new(collect: ChunkCollect<ADD_KINDS>, std: Arc<Std<F>>) -> Self {
        let frops_table_id = std
            .get_virtual_table_id(BinaryBasicFrops::TABLE_ID)
            .expect("Failed to get FROPS table ID");
        Self { inputs: Vec::new(), cursor: BinaryCollectCursor::new(collect), frops_table_id, std }
    }

    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        let op_data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        if OperationBusData::get_op(&op_data) != ZiskOp::Add.code() {
            return true;
        }

        let frops_row = BinaryBasicFrops::get_row(ZiskOp::Add.code(), data[A], data[B]);
        let kind = match add_shape(data[A], data[B]) {
            AddShape::Hi | AddShape::HiNeg => KIND_ADD_HI,
            AddShape::Full => KIND_ADD_FULL,
        };

        match self.cursor.next(kind, frops_row != BinaryBasicFrops::NO_FROPS) {
            CollectAction::Stop => false,
            CollectAction::Pass => true,
            CollectAction::CountFrop => {
                self.std.inc_virtual_row_one(self.frops_table_id, frops_row);
                true
            }
            CollectAction::Collect => {
                self.inputs
                    .push([OperationBusData::get_a(&op_data), OperationBusData::get_b(&op_data)]);
                !self.cursor.is_done()
            }
        }
    }
}

impl<F: PrimeField64> BusDevice<u64> for BinaryAddHiCollector<F> {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
