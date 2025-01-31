//! The `KeccakfInstance` module defines an instance to perform the witness computation
//! for the Keccakf State Machine.
//!
//! It manages collected inputs and interacts with the `KeccakfSM` to compute witnesses for
//! execution plans.

use crate::KeccakfSM;
use data_bus::{BusDevice, BusId, ExtOperationData, OperationBusData, OperationKeccakData};
use p3_field::PrimeField64;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, CollectSkipper, Instance, InstanceCtx, InstanceType};
use zisk_core::ZiskOperationType;
use std::sync::Arc;

/// The `KeccakfInstance` struct represents an instance for the Keccakf State Machine.
///
/// It encapsulates the `KeccakfSM` and its associated context, and it processes input data
/// to compute witnesses for the Keccakf State Machine.
pub struct KeccakfInstance {
    /// Keccakf state machine.
    keccakf_sm: Arc<KeccakfSM>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Helper to manage instruction skipping.
    collect_skipper: CollectSkipper,

    /// Collected inputs for witness computation.
    inputs: Vec<OperationKeccakData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,
}

impl KeccakfInstance {
    /// Creates a new `KeccakfInstance`.
    ///
    /// # Arguments
    /// * `keccakf_sm` - An `Arc`-wrapped reference to the Keccakf State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `KeccakfInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(keccakf_sm: Arc<KeccakfSM>, mut ictx: InstanceCtx, bus_id: BusId) -> Self {
        let collect_info = ictx.plan.collect_info.take().expect("collect_info should be Some");
        let collect_skipper =
            *collect_info.downcast::<CollectSkipper>().expect("Expected CollectSkipper");

        Self { keccakf_sm, ictx, collect_skipper, inputs: Vec::new(), bus_id }
    }
}

impl<F: PrimeField64> Instance<F> for KeccakfInstance {
    /// Computes the witness for the keccakf execution plan.
    ///
    /// This method leverages the `KeccakfSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(&mut self, _pctx: Option<&ProofCtx<F>>) -> Option<AirInstance<F>> {
        Some(self.keccakf_sm.compute_witness(&self.inputs))
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl BusDevice<u64> for KeccakfInstance {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether further processing should continue.
    /// - The second element contains derived inputs to be sent back to the bus (always empty).
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Keccak as u32 {
            return (false, vec![]);
        }

        if self.collect_skipper.should_skip() {
            return (false, vec![]);
        }

        if let ExtOperationData::OperationKeccakData(data) = data {
            self.inputs.push(data);

            // Check if the required number of inputs has been collected for computation.
            (self.inputs.len() == self.keccakf_sm.num_available_keccakfs, vec![])
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
