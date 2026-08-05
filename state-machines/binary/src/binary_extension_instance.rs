//! The `BinaryExtensionInstance` module defines an instance to perform witness computations
//! for binary extension operations using the Binary Extension State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryExtensionSM` to compute witnesses for
//! execution plans.

use crate::{BinaryExtensionCollector, BinaryExtensionSM, ChunkCollect, EXT_KINDS};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{
    BinaryExtensionFullTrace, BinaryExtensionFullTraceRow, BinaryExtensionFullTraceRowPacked,
    BinaryExtensionTrace, BinaryExtensionTraceRow, BinaryExtensionTraceRowPacked,
};

/// The `BinaryExtensionInstance` struct represents an instance for binary extension-related witness
/// computations.
///
/// It encapsulates the `BinaryExtensionSM` and its associated context, and it processes input data
/// to compute witnesses for binary extension operations.
pub struct BinaryExtensionInstance<F: PrimeField64> {
    /// Binary Extension state machine.
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// What this instance takes from each chunk: a `(count, skip)` per kind of operation, plus the
    /// frequent operations it accounts for.
    collect_info: HashMap<ChunkId, ChunkCollect<EXT_KINDS>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
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
    pub fn new(
        binary_extension_sm: Arc<BinaryExtensionSM<F>>,
        mut ictx: InstanceCtx,
        std: Arc<Std<F>>,
    ) -> Self {
        assert!(
            ictx.plan.air_id == BinaryExtensionTrace::<()>::AIR_ID
                || ictx.plan.air_id == BinaryExtensionFullTrace::<()>::AIR_ID,
            "BinaryExtensionInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, ChunkCollect<EXT_KINDS>>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { binary_extension_sm, collect_info, ictx, std }
    }

    /// `true` when this instance proves the full air (and therefore owns the dirty shapes).
    fn is_full(&self) -> bool {
        self.ictx.plan.air_id == BinaryExtensionFullTrace::<()>::AIR_ID
    }

    pub fn build_binary_extension_collector(
        &self,
        chunk_id: ChunkId,
    ) -> BinaryExtensionCollector<F> {
        BinaryExtensionCollector::new(self.collect_info[&chunk_id], self.std.clone())
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
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let _collector =
                    collector.as_any().downcast::<BinaryExtensionCollector<F>>().unwrap();
                _collector.inputs
            })
            .collect();

        // The two airs have different column sets, so the row type selects which one is filled.
        match (self.is_full(), packed) {
            (false, true) => {
                Ok(Some(self.binary_extension_sm.compute_witness::<BinaryExtensionTrace<
                    BinaryExtensionTraceRowPacked<F>,
                >, BinaryExtensionTraceRowPacked<F>>(
                    &inputs, trace_buffer
                )?))
            }
            (false, false) => {
                Ok(Some(self.binary_extension_sm.compute_witness::<BinaryExtensionTrace<
                    BinaryExtensionTraceRow<F>,
                >, BinaryExtensionTraceRow<F>>(
                    &inputs, trace_buffer
                )?))
            }
            (true, true) => {
                Ok(Some(self.binary_extension_sm.compute_witness::<BinaryExtensionFullTrace<
                    BinaryExtensionFullTraceRowPacked<F>,
                >, BinaryExtensionFullTraceRowPacked<F>>(
                    &inputs, trace_buffer
                )?))
            }
            (true, false) => {
                Ok(Some(self.binary_extension_sm.compute_witness::<BinaryExtensionFullTrace<
                    BinaryExtensionFullTraceRow<F>,
                >, BinaryExtensionFullTraceRow<F>>(
                    &inputs, trace_buffer
                )?))
            }
        }
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

    fn stats_type(&self) -> StatsType {
        StatsType::Opcodes
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(BinaryExtensionCollector::new(
            self.collect_info[&chunk_id],
            self.std.clone(),
        )))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
