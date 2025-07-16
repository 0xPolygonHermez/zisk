//! The `BinaryAddCollector` struct represents an input collector for binary add operations.

use fields::PrimeField64;
use pil_std_lib::Std;
use std::{collections::VecDeque, mem::ManuallyDrop, sync::Arc};
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, OPERATION_BUS_ID,
};
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::BinaryAddTraceRow;

use crate::binary_add::BinaryAddSM;

/// The `BinaryAddCollector` struct represents an input collector for binary add operations.
pub struct BinaryAddCollector<F: PrimeField64> {
    /// PIL2 Standard library.
    std: Arc<Std<F>>,

    /// Range ID for range checks.
    range_id: usize,

    /// The number of operations to collect.
    pub num_operations: usize,

    /// Helper to skip instructions based on the plan's configuration.
    pub collect_skipper: CollectSkipper,

    /// Current index in the rows vector.
    idx: usize,

    /// Binary add trace slice rows.
    rows: ManuallyDrop<Vec<BinaryAddTraceRow<F>>>,
}

impl<F: PrimeField64> BinaryAddCollector<F> {
    /// Creates a new `BinaryAddCollector`.
    ///
    /// # Arguments
    /// * `std` - The PIL2 standard library.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - Helper to skip instructions based on the plan's configuration.
    /// * `rows` - The binary add trace slice rows.
    ///
    /// # Returns
    /// A new `BinaryAddCollector` instance initialized with the provided parameters.
    pub fn new(
        std: Arc<Std<F>>,
        num_operations: usize,
        collect_skipper: CollectSkipper,
        rows: ManuallyDrop<Vec<BinaryAddTraceRow<F>>>,
    ) -> Self {
        // Search the range check ID in the standard library.
        let range_id = std.get_range(0, 0xFFFF, None);

        Self { std, range_id, num_operations, collect_skipper, idx: 0, rows }
    }
}

impl<F: PrimeField64> BusDevice<u64> for BinaryAddCollector<F> {
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
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.idx >= self.num_operations {
            return false;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op = OperationBusData::get_op(&data);

        if op != ZiskOp::Add.code() {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let input = [OperationBusData::get_a(&data), OperationBusData::get_b(&data)];
        let (row, range_checks) = BinaryAddSM::<F>::process_slice(&input);

        self.rows[self.idx] = row;

        for range_check in range_checks {
            self.std.range_check(range_check as i64, 1, self.range_id);
        }

        self.idx += 1;

        self.idx < self.num_operations
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
