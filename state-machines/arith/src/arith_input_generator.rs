//! The `ArithInputGenerator` module defines a device for generating binary inputs derived from
//! arithmetic operations.
//!
//! It implements the `Instance` and `BusDevice` traits, facilitating input generation
//! for the `ArithFullSM` state machine based on data received over the bus.

use data_bus::{BusDevice, BusId, ExtOperationData, OperationBusData};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, Instance};
use zisk_core::ZiskOperationType;

use crate::ArithFullSM;

/// The `ArithInputGenerator` struct acts as an input generator for arithmetic-related operations.
///
/// It interacts with the `ArithFullSM` to generate necessary inputs for binary computations
/// by processing arithmetic data received from the data bus.
#[derive(Default)]
pub struct ArithInputGenerator {
    /// The connected bus ID.
    bus_id: BusId,
}

impl ArithInputGenerator {
    pub fn new(bus_id: BusId) -> Self {
        Self { bus_id }
    }
}

impl<F: PrimeField> Instance<F> for ArithInputGenerator {
    /// Retrieves the checkpoint associated with this generator.
    ///
    /// # Returns
    /// A `CheckPoint::None`, as this generator does not maintain any `CheckPoint`.
    fn check_point(&self) -> CheckPoint {
        CheckPoint::None
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType::Instance`, indicating this is of type instance.
    fn instance_type(&self) -> sm_common::InstanceType {
        sm_common::InstanceType::Instance
    }

    /// Computes the witness for this generator.
    ///
    /// This generator does not compute a witness and always returns `None`.
    ///
    /// # Arguments
    /// * `_` - The proof context (unused in this implementation).
    ///
    /// # Returns
    /// Always returns `None`.
    fn compute_witness(&mut self, _: Option<&ProofCtx<F>>) -> Option<AirInstance<F>> {
        None
    }
}

impl BusDevice<u64> for ArithInputGenerator {
    /// Processes data received on the bus and generates inputs for binary operations.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether processing should continue (`false` in this case).
    /// - The second element contains the derived inputs to be sent back to the bus.
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let input: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&input);

        if op_type as u32 != ZiskOperationType::Arith as u32 {
            return (false, vec![]);
        }

        if let ExtOperationData::OperationData(input) = input {
            let inputs = ArithFullSM::generate_inputs(&input)
                .into_iter()
                .map(|x| (*bus_id, x))
                .collect::<Vec<_>>();
            (false, inputs)
        } else {
            panic!("Expected ExtOperationData::OperationData");
        }
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }
}
