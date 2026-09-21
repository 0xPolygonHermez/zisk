//! The `BinaryExtensionCollector` struct represents an input collector for binary extension
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use crate::{
    BinaryCollectCursor, BinaryExtensionFrops, BinaryInput, ChunkCollect, CollectAction, EXT_KINDS,
    KIND_EXT,
};
use zisk_common::{
    BusDevice, BusId, ExtOperationData, OperationBusData, A, B, OP, OPERATION_BUS_ID,
};

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use std::sync::Arc;

use zisk_core::frops::frops_multiplicity_from_asm;
#[cfg(feature = "debug_frops")]
use zisk_core::frops::{frops_check_claim_row, frops_check_enabled, FROPS_BINARY_EXT_BASE};
use zisk_core::ZiskOperationType;

/// The `BinaryExtensionCollector` struct represents an input collector for binary extension
pub struct BinaryExtensionCollector<F: PrimeField64> {
    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,

    /// Decides, operation by operation, what belongs to this instance.
    cursor: BinaryCollectCursor<EXT_KINDS>,

    /// The table ID for the Binary Extension FROPS
    frops_table_id: usize,

    /// Whether this collector publishes the FROPS multiplicity column. False when the column comes
    /// from the ROM-histogram assembly instead, in which case publishing here as well would count
    /// every multiplicity twice. Read once, at construction: it is process state for the whole
    /// execution.
    publish_frops: bool,

    /// Whether to cross-check the reference column against the rows this collector would have
    /// published (`zisk_core::frops`): the assembly's on the ASM path, a Rust replay of the
    /// minimal traces on the emulated one. Debug only, and compiled out without the `debug_frops`
    /// feature so that neither the field nor the per-operation test below it survives.
    #[cfg(feature = "debug_frops")]
    check_frops: bool,

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

        // Where the FROPS multiplicity column comes from is fixed for the whole execution, so it is
        // resolved once here rather than per operation.
        let publish_frops = !frops_multiplicity_from_asm();
        #[cfg(feature = "debug_frops")]
        let check_frops = frops_check_enabled();
        Self {
            inputs: Vec::new(),
            cursor: BinaryCollectCursor::new(collect),
            frops_table_id,
            publish_frops,
            #[cfg(feature = "debug_frops")]
            check_frops,
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

        if OperationBusData::get_op_type(&op_data) as u32 != ZiskOperationType::BinaryE as u32 {
            return true;
        }

        // The table row is only needed to publish the multiplicity or to cross-check the
        // assembly's column. Otherwise all the cursor needs is whether the operation is a frequent
        // one, which is the same test without the row arithmetic.
        let (is_frop, frops_row) = if {
            #[cfg(feature = "debug_frops")]
            {
                self.publish_frops || self.check_frops
            }
            #[cfg(not(feature = "debug_frops"))]
            {
                self.publish_frops
            }
        } {
            let row = BinaryExtensionFrops::get_row(data[OP] as u8, data[A], data[B]);
            (row != BinaryExtensionFrops::NO_FROPS, row)
        } else {
            (
                BinaryExtensionFrops::is_frequent_op(data[OP] as u8, data[A], data[B]),
                BinaryExtensionFrops::NO_FROPS,
            )
        };

        match self.cursor.next(KIND_EXT, is_frop) {
            CollectAction::Stop => false,
            CollectAction::Pass => true,
            CollectAction::CountFrop => {
                if self.publish_frops {
                    self.std.inc_virtual_row_one(self.frops_table_id, frops_row);
                }
                #[cfg(feature = "debug_frops")]
                if self.check_frops {
                    frops_check_claim_row(FROPS_BINARY_EXT_BASE + frops_row as u64);
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

impl<F: PrimeField64> BusDevice<u64> for BinaryExtensionCollector<F> {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
