//! The `BinaryBasicInstance` module defines an instance to perform witness computations
//! for binary-related operations using the Binary Basic State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryBasicSM` to compute witnesses for
//! execution plans.

use crate::{binary_basic_table::BinaryBasicTableSM, BinaryBasicCollector, BinaryBasicSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
    PayloadType,
};

use zisk_pil::{BinaryTrace, BinaryTraceSplit};

/// The `BinaryBasicInstance` struct represents an instance for binary-related witness computations.
///
/// It encapsulates the `BinaryBasicSM` and its associated context, and it processes input data
/// to compute witnesses for binary operations.
pub struct BinaryBasicInstance<F: PrimeField64> {
    /// Binary Basic state machine.
    binary_basic_sm: Arc<BinaryBasicSM>,

    /// Binary Basic Table State Machine.
    binary_basic_table_sm: Arc<BinaryBasicTableSM>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Indicates whether the instance should include ADD operations.
    with_adds: bool,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, CollectSkipper)>,

    /// Split binary trace to share split data between collectors.
    pub trace_split: Mutex<Option<BinaryTraceSplit<F>>>,
}

impl<F: PrimeField64> BinaryBasicInstance<F> {
    /// Creates a new `BinaryBasicInstance`.
    ///
    /// # Arguments
    /// * `binary_basic_sm` - Binary Basic State Machine.
    /// * `binary_basic_table_sm` - Binary Basic Table State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `BinaryBasicInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(
        binary_basic_sm: Arc<BinaryBasicSM>,
        binary_basic_table_sm: Arc<BinaryBasicTableSM>,
        mut ictx: InstanceCtx,
    ) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            BinaryTrace::<usize>::AIR_ID,
            "BinaryBasicInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let (with_adds, collect_info) = *meta
            .downcast::<(bool, HashMap<ChunkId, (u64, CollectSkipper)>)>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self {
            binary_basic_sm,
            binary_basic_table_sm,
            ictx,
            with_adds,
            collect_info,
            trace_split: Mutex::new(None),
        }
    }
}

impl<F: PrimeField64> Instance<F> for BinaryBasicInstance<F> {
    /// Computes the witness for the binary execution plan.
    ///
    /// This method leverages the `BinaryBasicSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    /// * `_collectors` - A vector of input collectors to process and collect data for witness
    /// * `_buffer_pool` - A buffer pool for managing memory buffers during computation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        _buffer_pool: &dyn BufferPool<F>,
    ) -> Option<AirInstance<F>> {
        let local_tables: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector
                    .as_any()
                    .downcast::<BinaryBasicCollector<F>>()
                    .unwrap()
                    .binary_basic_local_table
            })
            .collect();

        for local in local_tables {
            self.binary_basic_table_sm.update_multiplicity_from_local_table(&local);
        }

        let split_struct = self.trace_split.lock().unwrap().take().unwrap();
        Some(self.binary_basic_sm.compute_witness(split_struct))
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
        let trace = BinaryTrace::new_from_vec(buffer);

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
        Some(Box::new(BinaryBasicCollector::new(
            num_ops as usize,
            collect_skipper,
            self.with_adds,
            rows,
        )))
    }
}
