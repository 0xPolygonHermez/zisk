//! The `BinaryExtensionInstance` module defines an instance to perform witness computations
//! for binary extension operations using the Binary Extension State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryExtensionSM` to compute witnesses for
//! execution plans.

use crate::{
    binary_extension_table::BinaryExtensionTableSM, BinaryExtensionCollector, BinaryExtensionSM,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
    PayloadType,
};
use zisk_pil::{BinaryExtensionTrace, BinaryExtensionTraceSplit};

/// The `BinaryExtensionInstance` struct represents an instance for binary extension-related witness
/// computations.
///
/// It encapsulates the `BinaryExtensionSM` and its associated context, and it processes input data
/// to compute witnesses for binary extension operations.
pub struct BinaryExtensionInstance<F: PrimeField64> {
    /// Binary Extension state machine.
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Binary Extension Table State Machine.
    binary_extension_table_sm: Arc<BinaryExtensionTableSM>,

    /// PIL2 Standard library.
    std: Arc<Std<F>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, CollectSkipper)>,

    /// Split binary trace to share split data between collectors.
    trace_split: Mutex<Option<BinaryExtensionTraceSplit<F>>>,
}

impl<F: PrimeField64> BinaryExtensionInstance<F> {
    /// Creates a new `BinaryExtensionInstance`.
    ///
    /// # Arguments
    /// * `binary_extension_sm` - Binary Extension State Machine.
    /// * `binary_extension_table_sm` - Binary Extension Table State Machine.
    /// * `std` - The PIL2 standard library.
    /// * `instance_context` - The `InstanceCtx` associated with this instance, containing the
    ///   execution plan.
    ///
    /// # Returns
    /// A new `BinaryExtensionInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(
        binary_extension_sm: Arc<BinaryExtensionSM<F>>,
        binary_extension_table_sm: Arc<BinaryExtensionTableSM>,
        std: Arc<Std<F>>,

        mut ictx: InstanceCtx,
    ) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            BinaryExtensionTrace::<F>::AIR_ID,
            "BinaryExtensionInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self {
            binary_extension_sm,
            binary_extension_table_sm,
            std,
            ictx,
            collect_info,
            trace_split: Mutex::new(None),
        }
    }
}

impl<F: PrimeField64> Instance<F> for BinaryExtensionInstance<F> {
    /// Computes the witness for the binary extension execution plan.
    ///
    /// This method leverages the `BinaryExtensionSM` to generate an `AirInstance` using the
    /// collected inputs.
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
        Some(self.binary_extension_sm.compute_witness(split_struct))
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

    fn pre_collect(&self, buffer_pool: &dyn proofman_common::BufferPool<F>) {
        let buffer = buffer_pool.take_buffer();
        let trace = BinaryExtensionTrace::new_from_vec(buffer);

        let mut sizes = vec![0; self.collect_info.keys().len()];

        let mut keys: Vec<_> = self.collect_info.keys().collect();
        keys.sort();

        // Step 2: Iterate in sorted key order
        for (idx, key) in keys.iter().enumerate() {
            let value = self.collect_info.get(key).unwrap();
            sizes[idx] = value.0 as usize;
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
        let rows = self.trace_split.lock().unwrap().as_mut().unwrap().chunks.remove(0);

        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(BinaryExtensionCollector::new(
            self.binary_extension_table_sm.clone(),
            self.std.clone(),
            num_ops as usize,
            collect_skipper,
            rows,
        )))
    }
}
