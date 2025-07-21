//! The `Sha256fCounter` module defines a counter for tracking sha256f-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::Sha256f` instructions.

use std::{collections::VecDeque, ops::Add};

use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Counter, Metrics, A, B, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;

use crate::generate_sha256f_mem_inputs;

/// The `Sha256fCounter` struct represents a counter that monitors and measures
/// sha256f-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct Sha256fCounterInputGen {
    /// Sha256f counter.
    counter: Counter,

    /// Bus device mode (counter or input generator).
    mode: BusDeviceMode,
}

impl Sha256fCounterInputGen {
    /// Creates a new instance of `Sha256fCounter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `Sha256fCounter` instance.
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
        (op_type == ZiskOperationType::Sha256).then_some(self.counter.inst_count)
    }
}

impl Metrics for Sha256fCounterInputGen {
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

impl Add for Sha256fCounterInputGen {
    type Output = Sha256fCounterInputGen;

    /// Combines two `Sha256fCounter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `Sha256fCounter` instance.
    /// * `other` - The second `Sha256fCounter` instance.
    ///
    /// # Returns
    /// A new `Sha256fCounter` with combined counters.
    fn add(self, other: Self) -> Sha256fCounterInputGen {
        Sha256fCounterInputGen { counter: &self.counter + &other.counter, mode: self.mode }
    }
}

impl BusDevice<u64> for Sha256fCounterInputGen {
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
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::Sha256 as u32 {
            return true;
        }

        let step_main = data[A];
        let addr_main = data[B] as u32;

        let only_counters = self.mode == BusDeviceMode::Counter;
        if only_counters {
            self.measure(data);
        }

        pending.extend(generate_sha256f_mem_inputs(addr_main, step_main, data, only_counters));

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
