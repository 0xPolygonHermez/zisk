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
use zisk_core::frops::{
    frops_cross_check_enabled, frops_cross_check_row, frops_multiplicity_from_asm,
    FROPS_BINARY_BASIC_BASE,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use std::sync::Arc;

/// The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
pub struct BinaryBasicCollector<F: PrimeField64> {
    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,

    /// Decides, operation by operation, what belongs to this instance.
    cursor: BinaryCollectCursor<ADD_KINDS>,

    /// The table ID for the Binary FROPS
    frops_table_id: usize,

    /// Whether this collector publishes the FROPS multiplicity column. False when the column comes
    /// from the ROM-histogram assembly instead, in which case publishing here as well would count
    /// every multiplicity twice. Read once, at construction: it is process state for the whole
    /// execution.
    publish_frops: bool,

    /// Whether to cross-check the assembly's column against the rows this collector would have
    /// published (`zisk_core::frops`). Debug only.
    cross_check_frops: bool,

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

        // Where the FROPS multiplicity column comes from is fixed for the whole execution, so it is
        // resolved once here rather than per operation.
        let publish_frops = !frops_multiplicity_from_asm();
        let cross_check_frops = frops_cross_check_enabled();

        Self {
            inputs: Vec::new(),
            cursor: BinaryCollectCursor::new(collect),
            frops_table_id,
            publish_frops,
            cross_check_frops,
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

        // The table row is only needed to publish the multiplicity or to cross-check the
        // assembly's column. Otherwise all the cursor needs is whether the operation is a frequent
        // one, which is the same test without the row arithmetic.
        let (is_frop, frops_row) = if self.publish_frops || self.cross_check_frops {
            let row = BinaryBasicFrops::get_row(data[OP] as u8, data[A], data[B]);
            (row != BinaryBasicFrops::NO_FROPS, row)
        } else {
            (
                BinaryBasicFrops::is_frequent_op(data[OP] as u8, data[A], data[B]),
                BinaryBasicFrops::NO_FROPS,
            )
        };

        match self.cursor.next(kind, is_frop) {
            CollectAction::Stop => false,
            CollectAction::Pass => true,
            CollectAction::CountFrop => {
                if self.publish_frops {
                    self.std.inc_virtual_row_one(self.frops_table_id, frops_row);
                }
                if self.cross_check_frops {
                    frops_cross_check_row(FROPS_BINARY_BASIC_BASE + frops_row as u64);
                }
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
