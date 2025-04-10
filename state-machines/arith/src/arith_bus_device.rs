//! The `ArithCounter` module defines a device for tracking and processing arithmetic-related operations
//! sent over the data bus. It serves a dual purpose:
//! - Counting arithmetic-related instructions (`ZiskOperationType::Arith`) and gathering metrics.
//! - Generating binary inputs derived from arithmetic operations for the `ArithFullSM` state machine.
//!
//! This module implements the `Metrics` and `BusDevice` traits, enabling seamless integration with
//! the system bus for both monitoring and input generation.

use data_bus::{BusDevice, BusId, ExtOperationData, OPERATION_BUS_ID, OP_TYPE};
use sm_common::{BusDeviceMode, Counter, Metrics};
use zisk_core::ZiskOperationType;

use crate::ArithFullSM;

/// The `ArithCounter` struct represents a counter that monitors and measures
/// arithmetic-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct ArithCounterInputGen {
    /// Vector of counters, one for each accepted `ZiskOperationType`.
    counter: Counter,

    /// Bus device mode (counter or input generator).
    mode: BusDeviceMode,
}

impl ArithCounterInputGen {
    /// Creates a new instance of `ArithCounter`.
    ///
    /// # Arguments
    /// * `mode` - The mode of the bus device.
    ///
    /// # Returns
    /// A new `ArithCounter` instance.
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { counter: Counter::default(), mode }
    }

    /// Retrieves the count of instructions for a specific `ZiskOperationType`.
    ///
    /// # Arguments
    /// * `op_type` - The operation type to retrieve the count for.
    ///
    /// # Returns
    /// Returns the count of instructions for the specified operation type.
    pub fn inst_count(&self, op_type: ZiskOperationType) -> Option<u64> {
        (op_type == ZiskOperationType::Arith).then_some(self.counter.inst_count)
    }
}

impl Metrics for ArithCounterInputGen {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs for the bus.
    #[inline(always)]
    fn measure(&mut self, _data: &[u64]) {
        self.counter.update(1);
    }

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl BusDevice<u64> for ArithCounterInputGen {
    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A vector of derived inputs to be sent back to the bus.
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::Arith as u32 {
            return None;
        }

        let data: ExtOperationData<u64> = data.try_into().ok()?;
        if let ExtOperationData::OperationData(data) = data {
            if self.mode == BusDeviceMode::Counter {
                self.measure(&data);
            }

            let bin_inputs = ArithFullSM::generate_inputs(&data);
            return Some(bin_inputs.into_iter().map(|x| (OPERATION_BUS_ID, x)).collect());
        }

        None
    }

    /// Returns the bus IDs associated with this counter.
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
