//! The `Poseidon2Counter` module defines a counter for tracking poseidon2-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::Poseidon2` instructions.

use std::ops::Add;

use precompiles_common::MemProcessor;

use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Counter, Metrics, A, B, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;

use crate::{generate_poseidon2_mem_inputs, skip_poseidon2_mem_inputs};

/// The `Poseidon2Counter` struct represents a counter that monitors and measures
/// poseidon2-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct Poseidon2CounterInputGen {
    /// Poseidon2 counter.
    counter: Counter,

    /// Bus device mode (counter or input generator).
    mode: BusDeviceMode,
}

impl Poseidon2CounterInputGen {
    /// Creates a new instance of `Poseidon2Counter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `Poseidon2Counter` instance.
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
        (op_type == ZiskOperationType::Poseidon2).then_some(self.counter.inst_count)
    }

    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    /// * `mem_processors` – A queue of mem_processors bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        mem_processors: &mut P,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::Poseidon2 as u32 {
            return true;
        }

        let step_main = data[A];
        let addr_main = data[B] as u32;

        match self.mode {
            BusDeviceMode::Counter => {
                self.measure(data);
                generate_poseidon2_mem_inputs(addr_main, step_main, data, true, mem_processors);
            }
            BusDeviceMode::CounterAsm => {
                self.measure(data);
            }
            BusDeviceMode::InputGenerator => {
                if skip_poseidon2_mem_inputs(addr_main, mem_processors) {
                    return true;
                }
                generate_poseidon2_mem_inputs(addr_main, step_main, data, false, mem_processors);
            }
        }

        true
    }
}

impl Metrics for Poseidon2CounterInputGen {
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

impl Add for Poseidon2CounterInputGen {
    type Output = Poseidon2CounterInputGen;

    /// Combines two `Poseidon2Counter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `Poseidon2Counter` instance.
    /// * `other` - The second `Poseidon2Counter` instance.
    ///
    /// # Returns
    /// A new `Poseidon2Counter` with combined counters.
    fn add(self, other: Self) -> Poseidon2CounterInputGen {
        Poseidon2CounterInputGen { counter: &self.counter + &other.counter, mode: self.mode }
    }
}

impl BusDevice<u64> for Poseidon2CounterInputGen {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
