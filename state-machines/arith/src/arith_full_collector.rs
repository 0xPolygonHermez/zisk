use fields::PrimeField64;
/// The `ArithInstanceCollector` struct represents an input collector for arithmetic state machines.
use std::{collections::VecDeque, mem::ManuallyDrop, sync::Arc};
use zisk_common::{BusDevice, BusId, CollectSkipper, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithTraceRow;

use crate::{
    arith_full::ArithFullSM, arith_operation::ArithOperation, arith_range_table::ArithRangeTableSM,
    arith_range_table_helpers::ArithRangeTableInputs, arith_table::ArithTableSM,
    arith_table_helpers::ArithTableInputs,
};

pub struct ArithCollector<F: PrimeField64> {
    arith_table_sm: Arc<ArithTableSM>,
    arith_range_table_sm: Arc<ArithRangeTableSM>,
    arith_table_inputs: ArithTableInputs,
    arith_range_table_inputs: ArithRangeTableInputs,

    /// The number of operations to collect.
    num_operations: usize,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,

    /// Current index in the rows vector.
    idx: usize,

    /// Arithmetic operation instance used for processing inputs. Declared here to avoid
    /// reallocation on each call.
    aop: ArithOperation,

    /// Binary trace slice rows.
    rows: ManuallyDrop<Vec<ArithTraceRow<F>>>,
}

impl<F: PrimeField64> ArithCollector<F> {
    /// Creates a new `ArithInstanceCollector`.
    ///
    /// # Arguments
    ///
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
    pub fn new(
        arith_table_sm: Arc<ArithTableSM>,
        arith_range_table_sm: Arc<ArithRangeTableSM>,
        num_operations: usize,
        collect_skipper: CollectSkipper,
        rows: ManuallyDrop<Vec<ArithTraceRow<F>>>,
    ) -> Self {
        let arith_table_inputs = ArithTableInputs::new();
        let arith_range_table_inputs = ArithRangeTableInputs::new();
        let aop = ArithOperation::new();

        Self {
            arith_table_sm,
            arith_range_table_sm,
            arith_table_inputs,
            arith_range_table_inputs,
            num_operations,
            collect_skipper,
            idx: 0,
            aop,
            rows,
        }
    }
}

impl<F: PrimeField64> BusDevice<u64> for ArithCollector<F> {
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

        if data[OP_TYPE] as u32 != ZiskOperationType::Arith as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let data: [u64; 4] = data.try_into().expect("Slice must have length 4");

        ArithFullSM::process_input(
            &mut self.arith_range_table_inputs,
            &mut self.arith_table_inputs,
            &mut self.aop,
            &data,
            &mut self.rows[self.idx],
        );
        self.idx += 1;

        if self.idx == self.num_operations {
            self.arith_table_sm.process_inputs(&self.arith_table_inputs);
            self.arith_range_table_sm.process_inputs(&self.arith_range_table_inputs);
        }

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
