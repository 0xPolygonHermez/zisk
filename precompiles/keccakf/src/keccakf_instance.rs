//! The `KeccakfInstance` module defines an instance to perform the witness computation
//! for the Keccakf State Machine.
//!
//! It manages collected inputs and interacts with the `KeccakfSM` to compute witnesses for
//! execution plans.

use crate::KeccakfSM;
use data_bus::{
    BusDevice, BusId, ExtOperationData, OperationBusData, OperationKeccakData, PayloadType,
};
use p3_field::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use sm_common::{
    BusDeviceWrapper, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
};
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_core::ZiskOperationType;
use zisk_pil::KeccakfTrace;

/// The `KeccakfInstance` struct represents an instance for the Keccakf State Machine.
///
/// It encapsulates the `KeccakfSM` and its associated context, and it processes input data
/// to compute witnesses for the Keccakf State Machine.
pub struct KeccakfInstance {
    /// Keccakf state machine.
    keccakf_sm: Arc<KeccakfSM>,

    /// Instance context.
    ictx: InstanceCtx,

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
    pub fn new(keccakf_sm: Arc<KeccakfSM>, ictx: InstanceCtx, bus_id: BusId) -> Self {
        Self { keccakf_sm, ictx, bus_id }
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
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                collector.detach_device().as_any().downcast::<KeccakfCollector>().unwrap().inputs
            })
            .collect();

        Some(self.keccakf_sm.compute_witness(sctx, &inputs))
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

    fn build_inputs_collector(&self, chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            KeccakfTrace::<F>::AIR_ID,
            "KeccakfInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        Some(Box::new(KeccakfCollector::new(
            self.bus_id,
            collect_info[&chunk_id].0,
            collect_info[&chunk_id].1,
        )))
    }
}

pub struct KeccakfCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<OperationKeccakData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl KeccakfCollector {
    /// Creates a new `KeccakfCollector`.
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

impl BusDevice<PayloadType> for KeccakfCollector {
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
        _bus_id: &BusId,
        data: &[PayloadType],
    ) -> Option<Vec<(BusId, Vec<PayloadType>)>> {
        if self.inputs.len() == self.num_operations as usize {
            return None;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Keccak as u32 {
            return None;
        }

        if self.collect_skipper.should_skip() {
            return None;
        }

        if let ExtOperationData::OperationKeccakData(data) = data {
            self.inputs.push(data);

            // Check if the required number of inputs has been collected for computation.
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
        vec![self.bus_id]
    }

    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
