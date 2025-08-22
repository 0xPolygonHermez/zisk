//! The `BinaryExtensionInstance` module defines an instance to perform witness computations
//! for binary extension operations using the Binary Extension State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryExtensionSM` to compute witnesses for
//! execution plans.

use crate::{BinaryExtensionCollector, BinaryExtensionSM};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, BufferPool, ProofCtx, SetupCtx};
use std::{collections::HashMap, sync::Arc};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
    PayloadType,
};
use zisk_pil::BinaryExtensionTrace;

/// The `BinaryExtensionInstance` struct represents an instance for binary extension-related witness
/// computations.
///
/// It encapsulates the `BinaryExtensionSM` and its associated context, and it processes input data
/// to compute witnesses for binary extension operations.
pub struct BinaryExtensionInstance<F: PrimeField64> {
    /// Binary Extension state machine.
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Collect info for each chunk ID, containing the number of rows and a skipper for collection.
    collect_info: HashMap<ChunkId, (u64, bool, CollectSkipper)>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> BinaryExtensionInstance<F> {
    /// Creates a new `BinaryExtensionInstance`.
    ///
    /// # Arguments
    /// * `binary_extension_sm` - An `Arc`-wrapped reference to the Binary Extension State Machine.
    /// * `instance_context` - The `InstanceCtx` associated with this instance, containing the
    ///   execution plan.
    ///
    /// # Returns
    /// A new `BinaryExtensionInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(binary_extension_sm: Arc<BinaryExtensionSM<F>>, mut ictx: InstanceCtx) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            BinaryExtensionTrace::<F>::AIR_ID,
            "BinaryExtensionInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, bool, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { binary_extension_sm, collect_info, ictx }
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
            let c: Box<BinaryExtensionCollector<F>> = collector.as_any().downcast().unwrap();
            if !c.calculate_inputs {
                return None;
            }
            inputs.push(c.inputs);
        }

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        self.compute_multiplicity_instance(total_inputs);
        Some(self.binary_extension_sm.compute_witness(&inputs, buffer_pool.take_buffer()))
    }

    fn compute_multiplicity_instance(&self, total_inputs: usize) {
        self.binary_extension_sm.compute_multiplicity_instance(total_inputs);
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
    fn build_inputs_collector(
        &self,
        std: Arc<Std<F>>,
        chunk_id: ChunkId,
    ) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let (num_ops, force_execute_to_end, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(BinaryExtensionCollector::new(
            std,
            num_ops as usize,
            collect_skipper,
            force_execute_to_end,
        )))
    }
}
