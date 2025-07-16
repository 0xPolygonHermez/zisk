//! The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use std::{collections::VecDeque, mem::ManuallyDrop, sync::Arc};

use crate::{binary_basic::BinaryBasicSM, binary_basic_table::BinaryBasicTableSM, BinaryInput};
use fields::PrimeField64;
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, OPERATION_BUS_ID,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_pil::BinaryTraceRow;

/// The `BinaryBasicCollector` struct represents an input collector for binary-related operations.
pub struct BinaryBasicCollector<F: PrimeField64> {
    /// Binary Basic Table State Machine.
    binary_basic_table_sm: Arc<BinaryBasicTableSM>,

    /// The number of operations to collect.
    pub num_operations: usize,

    /// Helper to skip instructions based on the plan's configuration.
    pub collect_skipper: CollectSkipper,

    /// Flag to indicate that this instance comute add operations
    with_adds: bool,

    /// Current index in the rows vector.
    idx: usize,

    /// Binary trace slice rows.
    rows: ManuallyDrop<Vec<BinaryTraceRow<F>>>,
}

impl<F: PrimeField64> BinaryBasicCollector<F> {
    /// Creates a new `BinaryBasicCollector`.
    ///
    /// # Arguments
    /// * `binary_basic_table_sm` - Binary Basic Table State Machine.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - Helper to skip instructions based on the plan's configuration.
    /// * `with_adds` - Flag to indicate that this instance computes add operations.
    /// * `rows` - The binary trace slice rows.
    ///
    /// # Returns
    /// A new `BinaryBasicCollector` instance initialized with the provided parameters.
    pub fn new(
        binary_basic_table_sm: Arc<BinaryBasicTableSM>,
        num_operations: usize,
        collect_skipper: CollectSkipper,
        with_adds: bool,
        rows: ManuallyDrop<Vec<BinaryTraceRow<F>>>,
    ) -> Self {
        Self { binary_basic_table_sm, num_operations, collect_skipper, with_adds, idx: 0, rows }
    }
}

impl<F: PrimeField64> BusDevice<u64> for BinaryBasicCollector<F> {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An optional vector of tuples where:
    /// - The first element is the bus ID.
    /// - The second element is always empty indicating there are no derived inputs.
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.idx >= self.num_operations {
            return;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
            return;
        }

        if !self.with_adds && OperationBusData::get_op(&data) == ZiskOp::Add.code() {
            return;
        }

        if self.collect_skipper.should_skip() {
            return;
        }

        let binary_input = BinaryInput::from(&data);
        self.rows[self.idx] =
            BinaryBasicSM::process_slice(&binary_input, &self.binary_basic_table_sm);
        self.idx += 1;
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
