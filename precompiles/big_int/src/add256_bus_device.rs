//! The `Add256Counter` module defines a counter for tracking add256-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::Add256` instructions.

use std::{collections::VecDeque, ops::Add};

use zisk_common::MemCollectorInfo;
use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Counter, Metrics, A, B, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;

use crate::{generate_add256_mem_inputs, skip_add256_mem_inputs};

/// The `Add256Counter` struct represents a counter that monitors and measures
/// add256-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct Add256CounterInputGen {
    /// Add256 counter.
    counter: Counter,

    /// Bus device mode (counter or input generator).
    mode: BusDeviceMode,
}

impl Add256CounterInputGen {
    /// Creates a new instance of `Add256Counter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `Add256Counter` instance.
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
        (op_type == ZiskOperationType::BigInt).then_some(self.counter.inst_count)
    }
}

impl Metrics for Add256CounterInputGen {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `_data` - The data received from the bus.
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

impl Add for Add256CounterInputGen {
    type Output = Add256CounterInputGen;

    /// Combines two `Add256Counter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `Add256Counter` instance.
    /// * `other` - The second `Add256Counter` instance.
    ///
    /// # Returns
    /// A new `Add256Counter` with combined counters.
    fn add(self, other: Self) -> Add256CounterInputGen {
        Add256CounterInputGen { counter: &self.counter + &other.counter, mode: self.mode }
    }
}

impl BusDevice<u64> for Add256CounterInputGen {
    /// Processes data received on the bus, updating counters and generating inputs when applicable.
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
        pending: &mut VecDeque<(BusId, Vec<u64>)>,
        mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::BigInt as u32 {
            return true;
        }

        if let Some(mem_collectors_info) = mem_collector_info {
            if skip_add256_mem_inputs(data[B] as u32, data, mem_collectors_info) {
                return true;
            }
        }

        let step_main = data[A];
        let addr_main = data[B] as u32;

        let only_counters = self.mode == BusDeviceMode::Counter;
        if only_counters {
            self.measure(data);
        }

        generate_add256_mem_inputs(addr_main, step_main, data, only_counters, pending);

        true
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
