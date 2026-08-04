//! The `BinaryAddHiCollector` struct represents an input collector for the packed add operations
//! proven by `BinaryAddHi`.

use crate::{
    add_shape, BinaryBasicFrops, BinaryCollectCursor, BinaryCollectInfo, CollectAction, ShapeDrop,
};
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, A, B, OPERATION_BUS_ID,
};
use zisk_core::zisk_ops::ZiskOp;

use fields::PrimeField64;
use pil_std_lib::Std;
use std::sync::Arc;

/// The `BinaryAddHiCollector` struct represents an input collector for packed add operations.
pub struct BinaryAddHiCollector<F: PrimeField64> {
    /// Collected inputs for witness computation.
    pub inputs: Vec<[u64; 2]>,

    /// Decides, operation by operation, what belongs to this instance.
    cursor: BinaryCollectCursor,

    /// The table ID for the Binary Add FROPS
    frops_table_id: usize,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryAddHiCollector<F> {
    /// Creates a new `BinaryAddHiCollector`.
    ///
    /// # Arguments
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - Helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `BinaryAddHiCollector` instance initialized with the provided parameters.
    pub fn new(
        num_operations: usize,
        collect_skipper: CollectSkipper,
        force_execute_to_end: bool,
        std: Arc<Std<F>>,
    ) -> Self {
        let frops_table_id = std
            .get_virtual_table_id(BinaryBasicFrops::TABLE_ID)
            .expect("Failed to get FROPS table ID");
        // This air only proves the low-limb shape, and always takes a prefix of it, so nothing of
        // that shape is ever dropped while the full one never belongs to it.
        Self {
            inputs: Vec::with_capacity(num_operations),
            cursor: BinaryCollectCursor::new(BinaryCollectInfo {
                count: num_operations as u64,
                skipper: collect_skipper,
                hi_drop: ShapeDrop::none(),
                full_drop: ShapeDrop::all(),
                force_execute_to_end,
            }),
            frops_table_id,
            std,
        }
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

        match self
            .cursor
            .next(Some(add_shape(data[A], data[B])), frops_row != BinaryBasicFrops::NO_FROPS)
        {
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
