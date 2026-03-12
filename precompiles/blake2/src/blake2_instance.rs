//! The `Blake2Instance` module defines an instance to perform the witness computation
//! for the Blake2 State Machine.
//!
//! It manages collected inputs and interacts with the `Blake2SM` to compute witnesses for
//! execution plans.

use crate::{Blake2Input, Blake2SM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_common::ChunkId;
use zisk_common::{
    BusDevice, BusId, CheckPoint, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, PayloadType, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;
use zisk_pil::Blake2brTrace;

/// The `Blake2Instance` struct represents an instance for the Blake2 State Machine.
///
/// It encapsulates the `Blake2SM` and its associated context, and it processes input data
/// to compute witnesses for the Blake2 State Machine.
pub struct Blake2Instance<F: PrimeField64> {
    /// Blake2 state machine.
    blake2_sm: Arc<Blake2SM<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> Blake2Instance<F> {
    /// Creates a new `Blake2Instance`.
    ///
    /// # Arguments
    /// * `blake2_sm` - An `Arc`-wrapped reference to the Blake2 State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Blake2Instance` instance initialized with the provided state machine and
    /// context.
    pub fn new(blake2_sm: Arc<Blake2SM<F>>, ictx: InstanceCtx) -> Self {
        Self { blake2_sm, ictx }
    }

    pub fn build_blake2_collector(&self, chunk_id: ChunkId) -> Blake2Collector {
        assert_eq!(
            self.ictx.plan.air_id,
            Blake2brTrace::<F>::AIR_ID,
            "Blake2Instance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Blake2Collector::new(num_ops, collect_skipper)
    }
}

impl<F: PrimeField64> Instance<F> for Blake2Instance<F> {
    /// Computes the witness for the blake2 execution plan.
    ///
    /// This method leverages the `Blake2SM` to generate an `AirInstance` using the collected
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
            .map(|(_, collector)| collector.as_any().downcast::<Blake2Collector>().unwrap().inputs)
            .collect();

        Ok(Some(self.blake2_sm.compute_witness(&inputs, trace_buffer)?))
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

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            Blake2brTrace::<F>::AIR_ID,
            "Blake2Instance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(Blake2Collector::new(num_ops, collect_skipper)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct Blake2Collector {
    /// Collected inputs for witness computation.
    inputs: Vec<Blake2Input>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl Blake2Collector {
    /// Creates a new `Blake2Collector`.
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

        if data[OP_TYPE] as u32 != ZiskOperationType::Blake2 as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        if let ExtOperationData::OperationBlake2Data(data) = data {
            self.inputs.push(Blake2Input::from(&data));
        } else {
            panic!("Expected ExtOperationData::OperationBlake2Data");
        }

        self.inputs.len() < self.num_operations as usize
    }
}

impl BusDevice<PayloadType> for Blake2Collector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
