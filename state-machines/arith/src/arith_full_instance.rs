//! The `ArithFullInstance` module defines an instance to perform witness computations
//! for arithmetic-related operations using the Arithmetic Full State Machine.
//!
//! It manages collected inputs and interacts with the `ArithFullSM` to compute witnesses for
//! execution plans.

use crate::ArithFullSM;
use data_bus::{BusDevice, BusId, OperationBusData, OperationData, PayloadType};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{
    BusDeviceWrapper, CheckPoint, CollectSkipper, Instance, InstanceCtx, InstanceType,
};
use std::sync::Arc;
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
    pub fn new(arith_full_sm: Arc<ArithFullSM>, ictx: InstanceCtx, bus_id: BusId) -> Self {
        Self { arith_full_sm, ictx, bus_id }
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
        None
        // Some(self.arith_full_sm.compute_witness(&self.inputs))
    }

    fn compute_witness2(
        &mut self,
        _pctx: &ProofCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let collectors = collectors
            .into_iter()
            .map(|(chunk_id, mut collector)| {
                let collector = collector
                    .detach_device()
                    .as_any()
                    .downcast::<ArithInstanceCollector>()
                    .unwrap();
                (chunk_id, collector)
            })
            .collect();

        Some(self.arith_full_sm.compute_witness2(collectors))
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

    fn build_inputs_collector2(&self, chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        match self.ictx.plan.air_id {
            ArithTrace::<F>::AIR_ID => {
                Some(Box::new(match &self.ictx.plan.check_point {
                    CheckPoint::Multiple2(check_point) => {
                        // check_point is an array
                        ArithInstanceCollector::new(
                            self.bus_id,
                            check_point[&chunk_id].0,
                            check_point[&chunk_id].1,
                        )
                    }
                    _ => panic!("Binary Basic: Invalid checkpoint type"),
                }))
            }
            _ => panic!("BinaryBasicInstance: Unsupported air_id: {:?}", self.ictx.plan.air_id),
        }
    }
}

pub struct ArithInstanceCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<OperationData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,

    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl ArithInstanceCollector {
    pub fn new(bus_id: BusId, num_operations: u64, collect_skipper: CollectSkipper) -> Self {
        Self { inputs: Vec::new(), bus_id, num_operations, collect_skipper }
    }
}

impl BusDevice<u64> for ArithInstanceCollector {
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
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        if self.inputs.len() == self.num_operations as usize {
            return None;
        }

        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Arith as u32 {
            return None;
        }

        if self.collect_skipper.should_skip() {
            return None;
        }

        self.inputs.push(data);

        // Check if the required number of rows has been collected for computation.
        None
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }

    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
