//! The `ArithFullInstance` module defines an instance to perform witness computations
//! for arithmetic-related operations using the Arithmetic Full State Machine.
//!
//! It manages collected inputs and interacts with the `ArithFullSM` to compute witnesses for
//! execution plans.

use crate::{ArithFrops, ArithFullSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use zisk_common::{
    BusDevice, BusId, CheckPoint, ChunkId, CollectSkipper, ExtOperationData, Instance, InstanceCtx,
    InstanceType, MemCollectorInfo, OperationData, PayloadType, A, B, OP, OPERATION_BUS_ID,
    OP_TYPE,
};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithTrace;

/// The `ArithFullInstance` struct represents an instance for arithmetic-related witness
/// computations.
///
/// It encapsulates the `ArithFullSM` and its associated context, and it processes input data
/// to compute the witnesses for the arithmetic operations.
pub struct ArithFullInstance<F: PrimeField64> {
    /// Reference to the Arithmetic Full State Machine.
    arith_full_sm: Arc<ArithFullSM<F>>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, u64, bool, CollectSkipper)>,

    /// The instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> ArithFullInstance<F> {
    /// Creates a new `ArithFullInstance`.
    ///
    /// # Arguments
    /// * `arith_full_sm` - An `Arc`-wrapped reference to the Arithmetic Full State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `ArithFullInstance` instance initialized with the provided state machine and context.
    pub fn new(arith_full_sm: Arc<ArithFullSM<F>>, mut ictx: InstanceCtx) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            ArithTrace::<F>::AIR_ID,
            "ArithFullInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, u64, bool, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { arith_full_sm, collect_info, ictx }
    }

    pub fn build_arith_collector(&self, chunk_id: ChunkId) -> ArithInstanceCollector {
        let (num_ops, num_freq_ops, force_execute_to_end, collect_skipper) =
            self.collect_info[&chunk_id];
        ArithInstanceCollector::new(num_ops, num_freq_ops, collect_skipper, force_execute_to_end)
    }
}

impl<F: PrimeField64> Instance<F> for ArithFullInstance<F> {
    /// Computes the witness for the arithmetic execution plan.
    ///
    /// This method leverages the `ArithFullSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `pctx` - The proof context, unused in this implementation.
    /// * `sctx` - The setup context, unused in this implementation.
    /// * `collectors` - A vector of input collectors to process and collect data for witness
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
            .map(|(_, collector)| {
                let _collector = collector.as_any().downcast::<ArithInstanceCollector>().unwrap();
                self.arith_full_sm.compute_frops(&_collector.frops_inputs);
                _collector.inputs
            })
            .collect();
        Some(self.arith_full_sm.compute_witness(&inputs, trace_buffer))
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

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let (num_ops, num_freq_ops, force_execute_to_end, collect_skipper) =
            self.collect_info[&chunk_id];
        Some(Box::new(ArithInstanceCollector::new(
            num_ops,
            num_freq_ops,
            collect_skipper,
            force_execute_to_end,
        )))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// The `ArithInstanceCollector` struct represents an input collector for arithmetic state machines.
pub struct ArithInstanceCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<OperationData<u64>>,
    /// Collected rows for FROPS
    frops_inputs: Vec<u32>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,

    /// Flag to indicate that force to execute to end of chunk
    force_execute_to_end: bool,
}

impl ArithInstanceCollector {
    /// Creates a new `ArithInstanceCollector`.
    ///
    /// # Arguments
    ///
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    /// * `force_execute_to_end` - A flag to indicate whether to force execution to the end of the chunk.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
    pub fn new(
        num_operations: u64,
        num_freq_ops: u64,
        collect_skipper: CollectSkipper,
        force_execute_to_end: bool,
    ) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_skipper,
            frops_inputs: Vec::with_capacity(num_freq_ops as usize),
            force_execute_to_end,
        }
    }
}

impl BusDevice<u64> for ArithInstanceCollector {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
        _mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);
        let instance_complete = self.inputs.len() == self.num_operations as usize;

        if instance_complete && !self.force_execute_to_end {
            return false;
        }

        if data[OP_TYPE] as u32 != ZiskOperationType::Arith as u32 {
            return true;
        }

        let frops_row = ArithFrops::get_row(data[OP] as u8, data[A], data[B]);

        if self.collect_skipper.should_skip_query(frops_row == ArithFrops::NO_FROPS) {
            return true;
        }

        if frops_row != ArithFrops::NO_FROPS {
            self.frops_inputs.push(frops_row as u32);
            return true;
        }

        if instance_complete {
            // instance complete => no FROPS operation => discard, inputs complete
            return true;
        }

        let data: ExtOperationData<u64> = data.try_into().expect("Failed to convert data");

        if let ExtOperationData::OperationData(data) = data {
            self.inputs.push(data);
        }

        self.inputs.len() < self.num_operations as usize || self.force_execute_to_end
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
