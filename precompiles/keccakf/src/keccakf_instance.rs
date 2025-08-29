//! The `KeccakfInstance` module defines an instance to perform the witness computation
//! for the Keccakf State Machine.
//!
//! It manages collected inputs and interacts with the `KeccakfSM` to compute witnesses for
//! execution plans.
use crate::{KeccakfInput, KeccakfSM};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use zisk_common::{
    BusDevice, BusId, CheckPoint, ChunkId, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, PayloadType, OPERATION_BUS_ID, OP_TYPE,
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
            KeccakfTrace::<usize>::AIR_ID,
            "KeccakfInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { keccakf_sm, collect_info, ictx }
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
        buffer_pool: &dyn BufferPool<F>,
    ) -> Option<AirInstance<F>> {
        let mut inputs = Vec::with_capacity(collectors.len());

        for (_, collector) in collectors {
            let c: Box<KeccakfCollector> = collector.as_any().downcast().unwrap();
            if !c.calculate_inputs {
                return None;
            }
            inputs.push(c.inputs);
        }

        Some(self.keccakf_sm.compute_witness(&inputs, buffer_pool.take_buffer()))
    }

    fn compute_multiplicity_instance(&self) {
        self.keccakf_sm.compute_multiplicity_instance();
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

    fn build_inputs_collector(
        &self,
        _std: Arc<Std<F>>,
        chunk_id: ChunkId,
        calculate_inputs: bool,
        calculate_multiplicity: bool,
    ) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            KeccakfTrace::<F>::AIR_ID,
            "KeccakfInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(KeccakfCollector::new(
            num_ops,
            calculate_inputs,
            calculate_multiplicity,
            collect_skipper,
        )))
    }
}

pub struct KeccakfCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<KeccakfInput>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,

    pub calculate_inputs: bool,

    pub calculate_multiplicity: bool,

    inputs_collected: usize,
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
    pub fn new(
        num_operations: u64,
        calculate_inputs: bool,
        calculate_multiplicity: bool,
        collect_skipper: CollectSkipper,
    ) -> Self {
        Self {
            inputs: Vec::new(),
            num_operations,
            collect_skipper,
            calculate_inputs,
            calculate_multiplicity,
            inputs_collected: 0,
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
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
        _pending: &mut VecDeque<(BusId, Vec<PayloadType>)>,
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
            self.inputs_collected += 1;
            if self.calculate_inputs {
                self.inputs.push(KeccakfInput::from(&data));
            }
        } else {
            panic!("Expected ExtOperationData::OperationData");
        }

        self.inputs_collected < self.num_operations as usize
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
