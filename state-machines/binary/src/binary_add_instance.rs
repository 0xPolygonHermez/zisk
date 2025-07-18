//! The `BinaryAddInstance` module defines an specific instance to perform witness computations
//! for binary add operations using the Binary Add State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryAddSM` to compute witnesses for
//! execution plans.

use crate::{BinaryAddCollector, BinaryAddSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::{collections::HashMap, sync::Arc};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
    PayloadType,
};
use zisk_pil::BinaryAddTrace;

/// The `BinaryAddInstance` struct represents an instance for binary add witness computations.
///
/// It encapsulates the `BinaryAddSM` and its associated context, and it processes input data
/// to compute witnesses for binary operations.
pub struct BinaryAddInstance<F: PrimeField64> {
    /// Binary Add state machine.
    binary_add_sm: Arc<BinaryAddSM<F>>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, CollectSkipper)>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> BinaryAddInstance<F> {
    /// Creates a new `BinaryAddInstance`.
    ///
    /// # Arguments
    /// * `binary_add_sm` - An `Arc`-wrapped reference to the Binary Add State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `BinaryAddInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(binary_add_sm: Arc<BinaryAddSM<F>>, mut ictx: InstanceCtx) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            BinaryAddTrace::<F>::AIR_ID,
            "BinaryAddInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { binary_add_sm, collect_info, ictx }
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
                collector.as_any().downcast::<BinaryAddCollector>().unwrap().inputs
            })
            .collect();

        Some(self.binary_add_sm.compute_witness(&inputs, trace_buffer))
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
        assert_eq!(
            self.ictx.plan.air_id,
            BinaryAddTrace::<F>::AIR_ID,
            "BinaryAddInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );
        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(BinaryAddCollector::new(num_ops as usize, collect_skipper)))
    }
}
