//! The `DmaCollector` module defines an collector to calculate all inputs of an instance
//! for the Dma State Machine.

use std::any::Any;

use zisk_common::{BusDevice, BusId, CollectCounter, ExtOperationData, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::ZiskOperationType;

use crate::DmaInput;

pub struct DmaCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<DmaInput>,

    /// The number of operations to collect.
    pub num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    pub collect_counter: CollectCounter,
}

impl DmaCollector {
    /// Creates a new `DmaCollector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
    pub fn new(num_operations: u64, collect_counter: CollectCounter) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_counter,
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
    /// A tuple where:
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64], data_ext: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return true;
        }

        if self.collect_counter.should_skip() {
            return true;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        if let ExtOperationData::OperationDmaMemCpyData(data) = data {
            self.inputs.push(DmaInput::from_memcpy(&data, data_ext));
        } else if let ExtOperationData::OperationDmaMemCmpData(data) = data {
            self.inputs.push(DmaInput::from_memcmp(&data, data_ext));
        } else {
            panic!("Expected ExtOperationData::OperationDmaData");
        }

        self.inputs.len() < self.num_operations as usize
    }
}

impl BusDevice<u64> for DmaCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
