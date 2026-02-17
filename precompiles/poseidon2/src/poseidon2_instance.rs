//! The `Poseidon2Instance` module defines an instance to perform the witness computation
//! for the Poseidon2 State Machine.
//!
//! It manages collected inputs and interacts with the `Poseidon2SM` to compute witnesses for
//! execution plans.

use crate::{Poseidon2Input, Poseidon2SM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_common::ChunkId;
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, BusId, CheckPoint, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, PayloadType, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;
use zisk_pil::Poseidon2Trace;

/// The `Poseidon2Instance` struct represents an instance for the Poseidon2 State Machine.
///
/// It encapsulates the `Poseidon2SM` and its associated context, and it processes input data
/// to compute witnesses for the Poseidon2 State Machine.
pub struct Poseidon2Instance<F: PrimeField64> {
    /// Poseidon2 state machine.
    poseidon2_sm: Arc<Poseidon2SM<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> Poseidon2Instance<F> {
    /// Creates a new `Poseidon2Instance`.
    ///
    /// # Arguments
    /// * `poseidon2_sm` - An `Arc`-wrapped reference to the Poseidon2 State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Poseidon2Instance` instance initialized with the provided state machine and
    /// context.
    pub fn new(poseidon2_sm: Arc<Poseidon2SM<F>>, ictx: InstanceCtx) -> Self {
        Self { poseidon2_sm, ictx }
    }

    pub fn build_poseidon2_collector(&self, chunk_id: ChunkId) -> Poseidon2Collector {
        assert_eq!(
            self.ictx.plan.air_id,
            Poseidon2Trace::<F>::AIR_ID,
            "Poseidon2Instance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Poseidon2Collector::new(num_ops, collect_skipper)
    }
}

impl<F: PrimeField64> Instance<F> for Poseidon2Instance<F> {
    /// Computes the witness for the poseidon2 execution plan.
    ///
    /// This method leverages the `Poseidon2SM` to generate an `AirInstance` using the collected
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
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<Poseidon2Collector>().unwrap().inputs
            })
            .collect();

        Ok(Some(self.poseidon2_sm.compute_witness(&inputs, trace_buffer)?))
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Precompiled
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            Poseidon2Trace::<F>::AIR_ID,
            "Poseidon2Instance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(Poseidon2Collector::new(num_ops, collect_skipper)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct Poseidon2Collector {
    /// Collected inputs for witness computation.
    inputs: Vec<Poseidon2Input>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl Poseidon2Collector {
    /// Creates a new `Poseidon2Collector`.
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
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_skipper,
        }
    }

    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A tuple where:
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[PayloadType]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        if data[OP_TYPE] as u32 != ZiskOperationType::Poseidon2 as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        if let ExtOperationData::OperationPoseidon2Data(data) = data {
            self.inputs.push(Poseidon2Input::from(&data));
        } else {
            panic!("Expected ExtOperationData::OperationData");
        }

        self.inputs.len() < self.num_operations as usize
    }
}

impl BusDevice<PayloadType> for Poseidon2Collector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
