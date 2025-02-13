//! The `ArithInputGenerator` module defines a device for generating binary inputs derived from
//! arithmetic operations.
//!
//! It implements the `Instance` and `BusDevice` traits, facilitating input generation
//! for the `ArithFullSM` state machine based on data received over the bus.

use data_bus::{BusDevice, BusId, ExtOperationData, OperationBusData, PayloadType, MEM_BUS_ID};
use zisk_core::ZiskOperationType;

use crate::KeccakfSM;

/// The `ArithInputGenerator` struct acts as an input generator for arithmetic-related operations.
///
/// It interacts with the `ArithFullSM` to generate necessary inputs for binary computations
/// by processing arithmetic data received from the data bus.
#[derive(Default)]
pub struct KeccakfInputGenerator {}

impl BusDevice<PayloadType> for KeccakfInputGenerator {
    /// Processes data received on the bus and generates inputs for memory operations.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether processing should continue (`false` in this case).
    /// - The second element contains the derived inputs to be sent back to the bus.
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        let data: ExtOperationData<u64> = data.try_into().ok()?;

        if OperationBusData::get_op_type(&data) as u32 != ZiskOperationType::Keccak as u32 {
            return None;
        }

        match data {
            ExtOperationData::OperationKeccakData(data) => {
                let mem_inputs = KeccakfSM::generate_inputs(&data);
                Some(mem_inputs.into_iter().map(|x| (MEM_BUS_ID, x)).collect())
            }
            _ => panic!("Expected ExtOperationData::OperationData"),
        }
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
