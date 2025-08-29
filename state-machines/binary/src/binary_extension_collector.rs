//! The `BinaryExtensionCollector` struct represents an input collector for binary extension
//!
//! It manages collected inputs for the `BinaryExtensionSM` to compute witnesses

use std::collections::VecDeque;

use crate::BinaryExtensionSM;
use crate::{BinaryExtensionFrops, BinaryInput};
use fields::PrimeField64;
use pil_std_lib::Std;
use std::sync::Arc;
use zisk_common::{
    BusDevice, BusId, CollectSkipper, ExtOperationData, OperationBusData, A, B, OP,
    OPERATION_BUS_ID,
};
use zisk_core::ZiskOperationType;

/// The `BinaryExtensionCollector` struct represents an input collector for binary extension
pub struct BinaryExtensionCollector<F: PrimeField64> {
    std: Arc<Std<F>>,

    /// Collected inputs for witness computation.
    pub inputs: Vec<BinaryInput>,

    pub num_operations: usize,
    pub collect_skipper: CollectSkipper,

    /// Flag to indicate that force to execute to end of chunk
    force_execute_to_end: bool,

    pub calculate_inputs: bool,

    pub calculate_multiplicity: bool,

    inputs_collected: usize,
}

impl<F: PrimeField64> BinaryExtensionCollector<F> {
    pub fn new(
        std: Arc<Std<F>>,
        num_operations: usize,
        calculate_inputs: bool,
        calculate_multiplicity: bool,
        collect_skipper: CollectSkipper,
        force_execute_to_end: bool,
    ) -> Self {
        Self {
            std,
            inputs: Vec::new(),
            num_operations,
            collect_skipper,
            force_execute_to_end,
            calculate_inputs,
            calculate_multiplicity,
            inputs_collected: 0,
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
        let instance_complete = self.inputs.len() == self.num_operations;

        if instance_complete && !self.force_execute_to_end {
            return false;
        }

        let frops_row = BinaryExtensionFrops::get_row(data[OP] as u8, data[A], data[B]);

        let op_data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&op_data);

        if op_type as u32 != ZiskOperationType::BinaryE as u32 {
            return true;
        }

        if self.collect_skipper.should_skip_query(frops_row == BinaryExtensionFrops::NO_FROPS) {
            return true;
        }

        if frops_row != BinaryExtensionFrops::NO_FROPS {
            if self.calculate_multiplicity {
                let frops_table_id = self.std.get_virtual_table_id(BinaryExtensionFrops::TABLE_ID);
                self.std.inc_virtual_row(frops_table_id, frops_row as u64, 1);
            }
            return true;
        }

        if instance_complete {
            // instance complete => no FROPS operation => discard, inputs complete
            return true;
        }

        let input = BinaryInput::from(&op_data);
        if self.calculate_multiplicity {
            BinaryExtensionSM::process_multiplicity(&self.std, &input);
        }

        self.inputs_collected += 1;
        if self.calculate_inputs {
            self.inputs.push(input);
        }

        self.inputs_collected < self.num_operations || self.force_execute_to_end
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
