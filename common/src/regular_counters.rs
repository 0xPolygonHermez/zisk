//! The `RegularCounters` module defines a generic counter for tracking operations
//! sent over the data bus. It is designed to be reusable across multiple state machines
//! and collects metrics for specified `ZiskOperationType` instructions.

use crate::{BusDevice, BusId, Counter, ExtOperationData, Metrics, OperationBusData};
use std::{collections::VecDeque, ops::Add};
use zisk_core::ZiskOperationType;

/// The `RegularCounters` struct represents a generic counter that monitors and measures
/// operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct RegularCounters {
    /// Vector of `ZiskOperationType` instructions to be counted.
    op_type: Vec<ZiskOperationType>,

    /// The connected bus ID.
    bus_id: BusId,

    /// Vector of counters, one for each accepted `ZiskOperationType`.
    counter: Vec<Counter>,
}

impl RegularCounters {
    /// Creates a new instance of `RegularCounters`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `RegularCounters` instance.
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

impl Metrics for RegularCounters {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs for the bus.
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        let data: ExtOperationData<u64> = data.try_into().unwrap_or_else(|_| {
            panic!(
                "Regular Metrics: Failed to convert data OP:0x{:X} len:{} data:{:?}",
                data[0],
                data.len(),
                data
            )
        });

        let inst_op_type = OperationBusData::get_op_type(&data);

        if let Some(index) = self.op_type.iter().position(|&op_type| op_type as u64 == inst_op_type)
        {
            self.counter[index].update(1);
        }
    }

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for RegularCounters {
    type Output = RegularCounters;

    /// Combines two `RegularCounters` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `RegularCounters` instance.
    /// * `other` - The second `RegularCounters` instance.
    ///
    /// # Returns
    /// A new `RegularCounters` with combined counters.
    fn add(self, other: Self) -> RegularCounters {
        let counter = self
            .counter
            .into_iter()
            .zip(other.counter)
            .map(|(counter, other_counter)| &counter + &other_counter)
            .collect();
        RegularCounters { bus_id: self.bus_id, op_type: self.op_type, counter }
    }
}

impl BusDevice<u64> for RegularCounters {
    /// Processes data received on the bus, updating counters.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(*bus_id == self.bus_id);

        self.measure(data);

        true
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
