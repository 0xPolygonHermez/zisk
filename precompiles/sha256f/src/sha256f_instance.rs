//! The `Sha256fInstance` module defines an instance to perform the witness computation
//! for the Sha256f State Machine.
//!
//! It manages collected inputs and interacts with the `Sha256fSM` to compute witnesses for
//! execution plans.

use crate::Sha256fSM;
use p3_field::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_common::ChunkId;
use zisk_common::{
    BusDevice, BusId, CheckPoint, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, OperationSha256Data, PayloadType, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;
use zisk_pil::Sha256fTrace;

/// The `Sha256fInstance` struct represents an instance for the Sha256f State Machine.
///
/// It encapsulates the `Sha256fSM` and its associated context, and it processes input data
/// to compute witnesses for the Sha256f State Machine.
pub struct Sha256fInstance {
    /// Sha256f state machine.
    sha256f_sm: Arc<Sha256fSM>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl Sha256fInstance {
    /// Creates a new `Sha256fInstance`.
    ///
    /// # Arguments
    /// * `sha256f_sm` - An `Arc`-wrapped reference to the Sha256f State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Sha256fInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(sha256f_sm: Arc<Sha256fSM>, ictx: InstanceCtx) -> Self {
        Self { sha256f_sm, ictx }
    }
}

impl<F: PrimeField64> Instance<F> for Sha256fInstance {
    /// Computes the witness for the sha256f execution plan.
    ///
    /// This method leverages the `Sha256fSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| collector.as_any().downcast::<Sha256fCollector>().unwrap().inputs)
            .collect();

        Some(self.sha256f_sm.compute_witness(sctx, &inputs))
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

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            Sha256fTrace::<F>::AIR_ID,
            "Sha256fInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(Sha256fCollector::new(num_ops, collect_skipper)))
    }
}

pub struct Sha256fCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<OperationSha256Data<u64>>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl Sha256fCollector {
    /// Creates a new `Sha256fCollector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
    pub fn new(num_operations: u64, collect_skipper: CollectSkipper) -> Self {
        Self { inputs: Vec::new(), num_operations, collect_skipper }
    }
}

impl BusDevice<PayloadType> for Sha256fCollector {
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
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> Option<Vec<(BusId, Vec<PayloadType>)>> {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return None;
        }

        if data[OP_TYPE] as u32 != ZiskOperationType::Sha256 as u32 {
            return None;
        }

        if self.collect_skipper.should_skip() {
            return None;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        if let ExtOperationData::OperationSha256Data(data) = data {
            self.inputs.push(data);
            None
        } else {
            panic!("Expected ExtOperationData::OperationData");
        }
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
