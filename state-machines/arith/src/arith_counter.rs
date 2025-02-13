//! The `ArithCounter` module defines a counter for tracking arithmetic-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::Arith` instructions.

use std::ops::Add;

use data_bus::{BusDevice, BusId, OperationBusData, OperationData};
use sm_common::{Counter, Metrics};
use zisk_core::ZiskOperationType;

use crate::ArithFullSM;

/// The `ArithCounter` struct represents a counter that monitors and measures
/// arithmetic-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct ArithCounter {
    /// Vector of `ZiskOperationType` instructions to be counted.
    op_type: Vec<ZiskOperationType>,

    /// The connected bus ID.
    bus_id: BusId,

    /// Vector of counters, one for each accepted `ZiskOperationType`.
    counter: Vec<Counter>,
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
    pub fn new(bus_id: BusId, op_type: Vec<ZiskOperationType>) -> Self {
        let counter = vec![Counter::default(); op_type.len()];
        Self { bus_id, op_type, counter }
    }

    /// Retrieves the count of instructions for a specific `ZiskOperationType`.
    ///
    /// # Arguments
    /// * `op_type` - The operation type to retrieve the count for.
    ///
    /// # Returns
    /// Returns the count of instructions for the specified operation type.
    pub fn inst_count(&self, op_type: ZiskOperationType) -> Option<u64> {
        if let Some(index) = self.op_type.iter().position(|&_op_type| op_type == _op_type) {
            return Some(self.counter[index].inst_count);
        }
        None
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
    fn measure(&mut self, data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let inst_op_type = OperationBusData::get_op_type(&data);
        if let Some(index) = self.op_type.iter().position(|&op_type| op_type as u64 == inst_op_type)
        {
            self.counter[index].update(1);
        }

        vec![]
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
        let counter = self
            .counter
            .into_iter()
            .zip(other.counter)
            .map(|(counter, other_counter)| &counter + &other_counter)
            .collect();
        ArithCounter { bus_id: self.bus_id, op_type: self.op_type, counter }
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
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        self.measure(data);

        let input: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&input);

        if op_type as u32 != ZiskOperationType::Arith as u32 {
            return None;
        }

        let inputs = ArithFullSM::generate_inputs(&input)
            .into_iter()
            .map(|x| (*bus_id, x))
            .collect::<Vec<_>>();

        Some(inputs)
    }

    /// Returns the bus IDs associated with this counter.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
