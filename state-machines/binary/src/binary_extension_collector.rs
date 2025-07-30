//! The `BinaryExtensionCollector` struct represents an input collector for binary extension
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use std::{collections::VecDeque, mem::ManuallyDrop, sync::Arc};

use crate::{
    binary_extension::BinaryExtensionSM, binary_extension_table::BinaryExtensionTableSM,
    BinaryInput,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, OPERATION_BUS_ID,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_pil::BinaryExtensionTraceRow;

/// The `BinaryExtensionCollector` struct represents an input collector for binary extension
pub struct BinaryExtensionCollector<F: PrimeField64> {
    /// Binary Extension Table State Machine.
    binary_extension_table_sm: Arc<BinaryExtensionTableSM>,

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

    /// Binary trace slice rows.
    rows: ManuallyDrop<Vec<BinaryExtensionTraceRow<F>>>,
}

impl<F: PrimeField64> BinaryExtensionCollector<F> {
    /// Creates a new `BinaryExtensionCollector`.
    ///
    /// # Arguments
    /// * `binary_extension_table_sm` - Binary Extension Table State Machine.
    /// * `std` - The PIL2 standard library.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - Helper to skip instructions based on the plan's configuration.
    /// * `rows` - The binary trace slice rows.
    ///
    /// # Returns
    /// A new `BinaryExtensionCollector` instance initialized with the provided parameters.
    pub fn new(
        binary_extension_table_sm: Arc<BinaryExtensionTableSM>,
        std: Arc<Std<F>>,
        num_operations: usize,
        collect_skipper: CollectSkipper,
        rows: ManuallyDrop<Vec<BinaryExtensionTraceRow<F>>>,
    ) -> Self {
        // Search the range check ID in the standard library.
        let range_id = std.get_range(0, 0xFFFFFF, None);

        Self {
            binary_extension_table_sm,
            std,
            range_id,
            num_operations,
            collect_skipper,
            idx: 0,
            rows,
        }
    }
}

impl<F: PrimeField64> BusDevice<u64> for BinaryExtensionCollector<F> {
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

        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::BinaryE as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let binary_input = BinaryInput::from(&data);

        BinaryExtensionSM::process_input(
            &binary_input,
            &self.binary_extension_table_sm,
            &mut self.rows[self.idx],
        );

        let opcode = ZiskOp::try_from_code(binary_input.op).expect("Invalid ZiskOp opcode");
        let op_is_shift = BinaryExtensionSM::<F>::opcode_is_shift(opcode);
        if op_is_shift {
            let row = (binary_input.b >> 8) & 0xFFFFFF;
            self.std.range_check(row as i64, 1, self.range_id);
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
