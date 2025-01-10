//! The `ArithFullInstance` module defines an instance to perform witness computations
//! for arithmetic-related operations using the Arithmetic Full State Machine.
//!
//! It manages collected inputs and interacts with the `ArithFullSM` to compute witnesses for
//! execution plans.

use crate::ArithFullSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, CollectSkipper, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithTrace;

/// The `ArithFullInstance` struct represents an instance for arithmetic-related witness
/// computations.
///
/// It encapsulates the `ArithFullSM` and its associated context, and it processes input data
/// to compute the witnesses for the arithmetic operations.
pub struct ArithFullInstance {
    /// Reference to the Arithmetic Full State Machine.
    arith_full_sm: Arc<ArithFullSM>,

    /// The instance context.
    ictx: InstanceCtx,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,

    /// Collected inputs for witness computation.
    inputs: Vec<OperationData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,
}

impl ArithFullInstance {
    /// Creates a new `ArithFullInstance`.
    ///
    /// # Arguments
    /// * `arith_full_sm` - An `Arc`-wrapped reference to the Arithmetic Full State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `ArithFullInstance` instance initialized with the provided state machine and context.
    pub fn new(arith_full_sm: Arc<ArithFullSM>, mut ictx: InstanceCtx, bus_id: BusId) -> Self {
        let collect_info = ictx.plan.collect_info.take().expect("collect_info should be Some");
        let collect_skipper =
            *collect_info.downcast::<CollectSkipper>().expect("Expected CollectSkipper");

        Self { arith_full_sm, ictx, collect_skipper, inputs: Vec::new(), bus_id }
    }
}

impl<F: PrimeField> Instance<F> for ArithFullInstance {
    /// Computes the witness for the arithmetic execution plan.
    ///
    /// This method leverages the `ArithFullSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.arith_full_sm.compute_witness::<F>(&self.inputs))
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

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }
}

impl BusDevice<u64> for ArithFullInstance {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether further processing should continue.
    /// - The second element is always empty.
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Arith as u32 {
            return (false, vec![]);
        }

        if self.collect_skipper.should_skip() {
            return (false, vec![]);
        }

        self.inputs.push(data);

        // Check if the required number of rows has been collected for computation.
        (self.inputs.len() == ArithTrace::<usize>::NUM_ROWS, vec![])
    }
}
