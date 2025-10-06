//! The `KeccakfInstance` module defines an instance to perform the witness computation
//! for the Keccakf State Machine.
//!
//! It manages collected inputs and interacts with the `KeccakfSM` to compute witnesses for
//! execution plans.
use crate::{KeccakfInput, KeccakfSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use zisk_common::{
    BusDevice, BusId, CheckPoint, ChunkId, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, MemCollectorInfo, PayloadType, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;
use zisk_pil::KeccakfTrace;

/// The `KeccakfInstance` struct represents an instance for the Keccakf State Machine.
///
/// It encapsulates the `KeccakfSM` and its associated context, and it processes input data
/// to compute witnesses for the Keccakf State Machine.
pub struct KeccakfInstance<F: PrimeField64> {
    /// Keccakf state machine.
    keccakf_sm: Arc<KeccakfSM<F>>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, CollectSkipper)>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> KeccakfInstance<F> {
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
    pub fn new(keccakf_sm: Arc<KeccakfSM<F>>, mut ictx: InstanceCtx) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            KeccakfTrace::<F>::AIR_ID,
            "KeccakfInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { keccakf_sm, collect_info, ictx }
    }

    pub fn build_keccakf_collector(&self, chunk_id: ChunkId) -> KeccakfCollector {
        assert_eq!(
            self.ictx.plan.air_id,
            KeccakfTrace::<F>::AIR_ID,
            "KeccakfInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        KeccakfCollector::new(num_ops, collect_skipper)
    }
}

impl<F: PrimeField64> Instance<F> for KeccakfInstance<F> {
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
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| collector.as_any().downcast::<KeccakfCollector>().unwrap().inputs)
            .collect();

        Some(self.keccakf_sm.compute_witness(&inputs, trace_buffer))
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
            KeccakfTrace::<F>::AIR_ID,
            "KeccakfInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(KeccakfCollector::new(num_ops, collect_skipper)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct KeccakfCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<KeccakfInput>,

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
    pub fn new(num_operations: u64, collect_skipper: CollectSkipper) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_skipper,
        }
    }
}

impl BusDevice<PayloadType> for KeccakfCollector {
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
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
        _pending: &mut VecDeque<(BusId, Vec<PayloadType>)>,
        _mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        if data[OP_TYPE] as u32 != ZiskOperationType::Keccak as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        if let ExtOperationData::OperationKeccakData(data) = data {
            self.inputs.push(KeccakfInput::from(&data));
        } else {
            panic!("Expected ExtOperationData::OperationData");
        }

        self.inputs.len() < self.num_operations as usize
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
