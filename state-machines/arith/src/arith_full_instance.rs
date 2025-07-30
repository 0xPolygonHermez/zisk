//! The `ArithFullInstance` module defines an instance to perform witness computations
//! for arithmetic-related operations using the Arithmetic Full State Machine.
//!
//! It manages collected inputs and interacts with the `ArithFullSM` to compute witnesses for
//! execution plans.

use crate::{arith_full_collector::ArithCollector, ArithFullSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
use std::sync::{Arc, Mutex};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, ChunkPlansMap, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{ArithTrace, ArithTraceSplit};

/// The `ArithFullInstance` struct represents an instance for arithmetic-related witness
/// computations.
///
/// It encapsulates the `ArithFullSM` and its associated context, and it processes input data
/// to compute the witnesses for the arithmetic operations.
pub struct ArithFullInstance<F: PrimeField64> {
    /// Reference to the Arithmetic Full State Machine.
    arith_full_sm: Arc<ArithFullSM>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: ChunkPlansMap,

    /// The instance context.
    ictx: InstanceCtx,

    /// Split arith trace to share split data between collectors.
    pub trace_split: Mutex<Option<ArithTraceSplit<F>>>,
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
    pub fn new(arith_full_sm: Arc<ArithFullSM>, mut ictx: InstanceCtx) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            ArithTrace::<usize>::AIR_ID,
            "ArithFullInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<ChunkPlansMap>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { arith_full_sm, collect_info, ictx, trace_split: Mutex::new(None) }
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
        _collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        _buffer_pool: &dyn BufferPool<F>,
    ) -> Option<AirInstance<F>> {
        let split_struct = self.trace_split.lock().unwrap().take().unwrap();
        Some(self.arith_full_sm.compute_witness(split_struct))
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

    fn pre_collect(&self, buffer_pool: &dyn proofman_common::BufferPool<F>) {
        let buffer = buffer_pool.take_buffer();
        let trace = ArithTrace::new_from_vec(buffer);

        let mut sizes = vec![0; self.collect_info.keys().len()];

        let mut keys: Vec<_> = self.collect_info.keys().collect();
        keys.sort();

        // Step 2: Iterate in sorted key order
        for (idx, key) in keys.iter().enumerate() {
            let chunk_plan = self.collect_info.get(key).unwrap();
            sizes[idx] = chunk_plan.num_ops as usize;
        }

        *self.trace_split.lock().unwrap() = Some(trace.to_split_struct(&sizes));
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let chunk_plan = &self.collect_info[&chunk_id];

        let rows = {
            let mut trace_split_guard = self.trace_split.lock().unwrap();
            let trace_split = trace_split_guard.as_mut().unwrap();
            std::mem::take(&mut trace_split.chunks[chunk_plan.idx as usize])
        };

        Some(Box::new(ArithCollector::new(
            self.arith_full_sm.arith_table_sm.clone(),
            self.arith_full_sm.arith_range_table_sm.clone(),
            chunk_plan.num_ops as usize,
            chunk_plan.skipper,
            rows,
        )))
    }
}
