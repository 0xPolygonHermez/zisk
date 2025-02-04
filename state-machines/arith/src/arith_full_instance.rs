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
    BusDeviceWrapper, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
};
use std::{collections::HashMap, sync::Arc};
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
    /// * `pctx` - The proof context, unused in this implementation.
    /// * `collectors` - A vector of input collectors to process and collect data for witness
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                collector
                    .detach_device()
                    .as_any()
                    .downcast::<ArithInstanceCollector>()
                    .unwrap()
                    .inputs
            })
            .collect();

        Some(self.arith_full_sm.compute_witness(&inputs))
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

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            ArithTrace::<F>::AIR_ID,
            "BinaryInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        Some(Box::new(ArithInstanceCollector::new(
            self.bus_id,
            collect_info[&chunk_id].0,
            collect_info[&chunk_id].1,
        )))
    }
}

/// The `ArithInstanceCollector` struct represents an input collector for arithmetic state machines.
pub struct ArithInstanceCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<OperationData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl ArithInstanceCollector {
    /// Creates a new `ArithInstanceCollector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
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
    /// An optional vector of tuples where:
    /// - The first element is the bus ID.
    /// - The second element is always empty indicating there are no derived inputs.
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

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
