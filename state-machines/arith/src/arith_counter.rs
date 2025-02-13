//! The `ArithCounter` module defines a counter for tracking arithmetic-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::Arith` instructions.

use std::ops::Add;

use data_bus::{BusDevice, BusId, OperationBusData, OperationData, OPERATION_BUS_ID};
use sm_common::{Counter, Metrics};
use zisk_core::ZiskOperationType;

use crate::ArithFullSM;

/// The `ArithCounter` struct represents a counter that monitors and measures
/// arithmetic-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct ArithCounter {
    /// Vector of counters, one for each accepted `ZiskOperationType`.
    counter: Counter,
}

impl ArithCounter {
    /// Creates a new instance of `ArithCounter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `ArithCounter` instance.
    pub fn new() -> Self {
        Self { counter: Counter::default() }
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

impl Metrics for ArithCounter {
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

impl Add for ArithCounter {
    type Output = ArithCounter;

    /// Combines two `ArithCounter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `ArithCounter` instance.
    /// * `other` - The second `ArithCounter` instance.
    ///
    /// # Returns
    /// A new `ArithCounter` with combined counters.
    fn add(self, other: Self) -> ArithCounter {
        ArithCounter { counter: &self.counter + &other.counter }
    }
}

impl BusDevice<u64> for ArithCounter {
    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A vector of derived inputs to be sent back to the bus.
    #[inline]
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        let data: OperationData<u64> = data.try_into().ok()?;

        if OperationBusData::get_op_type(&data) as u32 != ZiskOperationType::Arith as u32 {
            return None;
        }

        self.measure(&data);

        let bin_inputs = ArithFullSM::generate_inputs(&data);
        Some(bin_inputs.into_iter().map(|x| (OPERATION_BUS_ID, x)).collect())
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
