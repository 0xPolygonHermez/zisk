//! The `BinaryAddInstance` module defines an specific instance to perform witness computations
//! for binary add operations using the Binary Add State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryAddSM` to compute witnesses for
//! execution plans.

use crate::{BinaryAddCollector, BinaryAddSM};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
use std::sync::{Arc, Mutex};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, ChunkPlansMap, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{BinaryAddTrace, BinaryAddTraceSplit};

/// The `BinaryAddInstance` struct represents an instance for binary add witness computations.
///
/// It encapsulates the `BinaryAddSM` and its associated context, and it processes input data
/// to compute witnesses for binary operations.
pub struct BinaryAddInstance<F: PrimeField64> {
    /// Binary Add state machine.
    binary_add_sm: Arc<BinaryAddSM<F>>,

    /// PIL2 Standard library.
    std: Arc<Std<F>>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: ChunkPlansMap,

    /// Instance context.
    ictx: InstanceCtx,

    /// Split binary add trace to share split data between collectors.
    trace_split: Mutex<Option<BinaryAddTraceSplit<F>>>,
}

impl<F: PrimeField64> BinaryAddInstance<F> {
    /// Creates a new `BinaryAddInstance`.
    ///
    /// # Arguments
    /// * `binary_add_sm` - An `Arc`-wrapped reference to the Binary Add State Machine.
    /// * `std` - The PIL2 standard library.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `BinaryAddInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(
        binary_add_sm: Arc<BinaryAddSM<F>>,
        std: Arc<Std<F>>,
        mut ictx: InstanceCtx,
    ) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            BinaryAddTrace::<F>::AIR_ID,
            "BinaryAddInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<ChunkPlansMap>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { binary_add_sm, std, collect_info, ictx, trace_split: Mutex::new(None) }
    }
}

impl<F: PrimeField64> Instance<F> for BinaryAddInstance<F> {
    /// Computes the witness for the binary execution plan.
    ///
    /// This method leverages the `BinaryAddSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    /// * `collectors` - A vector of input collectors to process and collect data for witness
    /// * `buffer_pool` - The buffer pool to manage memory buffers.
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
        Some(self.binary_add_sm.compute_witness(split_struct))
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
        let trace = BinaryAddTrace::new_from_vec(buffer);

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

        Some(Box::new(BinaryAddCollector::new(
            self.std.clone(),
            chunk_plan.num_ops as usize,
            chunk_plan.skipper,
            rows,
        )))
    }
}
