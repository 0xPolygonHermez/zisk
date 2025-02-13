//! The `KeccakfCounter` module defines a counter for tracking keccakf-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::Keccakf` instructions.

use std::ops::Add;

use data_bus::{
    BusDevice, BusId, ExtOperationData, OperationBusData, MEM_BUS_ID, OPERATION_BUS_ID,
};
use sm_common::{Counter, Metrics};
use zisk_core::ZiskOperationType;

use crate::KeccakfSM;

/// The `KeccakfCounter` struct represents a counter that monitors and measures
/// keccakf-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct KeccakfCounter {
    /// Keccakf counter.
    counter: Counter,
}

impl KeccakfCounter {
    /// Creates a new instance of `KeccakfCounter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `KeccakfCounter` instance.
    pub fn new() -> Self {
        Self { counter: Counter::default() }
    }
}

impl Metrics for KeccakfCounter {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `_data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs for the bus.
    fn measure(&mut self, _bus_id: &BusId, _data: &[u64]) -> Vec<(BusId, Vec<u64>)> {
        self.counter.update(1);

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

impl Add for KeccakfCounter {
    type Output = KeccakfCounter;

    /// Combines two `KeccakfCounter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `KeccakfCounter` instance.
    /// * `other` - The second `KeccakfCounter` instance.
    ///
    /// # Returns
    /// A new `KeccakfCounter` with combined counters.
    fn add(self, other: Self) -> KeccakfCounter {
        KeccakfCounter { counter: &self.counter + &other.counter }
    }
}

impl BusDevice<u64> for KeccakfCounter {
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
        let data: ExtOperationData<u64> = data.try_into().ok()?;

        if OperationBusData::get_op_type(&data) as u32 != ZiskOperationType::Keccak as u32 {
            return None;
        }

        match data {
            ExtOperationData::OperationKeccakData(data) => {
                self.measure(&OPERATION_BUS_ID, &data);

                let mem_inputs = KeccakfSM::generate_inputs(&data);
                Some(mem_inputs.into_iter().map(|x| (MEM_BUS_ID, x)).collect())
            }
            _ => panic!("Expected ExtOperationData::OperationData"),
        }
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
