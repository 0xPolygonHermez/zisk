//! The `BinaryAddHiInstance` module defines an specific instance to perform witness computations
//! for the packed add operations proven by the Binary Add Hi State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryAddHiSM` to compute witnesses for
//! execution plans.

use crate::{BinaryAddHiCollector, BinaryAddHiSM, ChunkCollect, ADD_KINDS};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{
    BinaryAddHiHugeTrace, BinaryAddHiHugeTraceRow, BinaryAddHiHugeTraceRowPacked,
    BinaryAddHiLargeTrace, BinaryAddHiLargeTraceRow, BinaryAddHiLargeTraceRowPacked,
    BinaryAddHiTrace, BinaryAddHiTraceRow, BinaryAddHiTraceRowPacked,
};

/// Air id of each `BinaryAddHi` air. Each packs a different number of operations per row, so each
/// has its own row type.
const AIR_ID: usize = BinaryAddHiTrace::<()>::AIR_ID;
const LARGE_AIR_ID: usize = BinaryAddHiLargeTrace::<()>::AIR_ID;
const HUGE_AIR_ID: usize = BinaryAddHiHugeTrace::<()>::AIR_ID;

/// The `BinaryAddHiInstance` struct represents an instance for packed add witness computations.
///
/// It encapsulates the `BinaryAddHiSM` and its associated context, and it processes input data
/// to compute witnesses for the additions whose result fits in the low 32-bit limb.
pub struct BinaryAddHiInstance<F: PrimeField64> {
    /// Binary Add Hi state machine.
    binary_add_hi_sm: Arc<BinaryAddHiSM<F>>,

    /// What this instance takes from each chunk: a `(count, skip)` per kind of operation, plus the
    /// frequent operations it accounts for. The counts are in operations, not rows, since one row
    /// holds LANES_X_ROW of them.
    collect_info: HashMap<ChunkId, ChunkCollect<ADD_KINDS>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryAddHiInstance<F> {
    /// Creates a new `BinaryAddHiInstance`.
    ///
    /// # Arguments
    /// * `binary_add_hi_sm` - An `Arc`-wrapped reference to the Binary Add Hi State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `BinaryAddHiInstance` initialized with the provided state machine and context.
    pub fn new(
        binary_add_hi_sm: Arc<BinaryAddHiSM<F>>,
        mut ictx: InstanceCtx,
        std: Arc<Std<F>>,
    ) -> Self {
        assert!(
            matches!(ictx.plan.air_id, AIR_ID | LARGE_AIR_ID | HUGE_AIR_ID),
            "BinaryAddHiInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, ChunkCollect<ADD_KINDS>>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { binary_add_hi_sm, collect_info, ictx, std }
    }

    /// Which of the three `BinaryAddHi` airs this instance is. They pack a different number of
    /// operations per row, so this picks the row type the trace is built with.
    fn air_id(&self) -> usize {
        self.ictx.plan.air_id
    }

    pub fn build_binary_add_hi_collector(&self, chunk_id: ChunkId) -> BinaryAddHiCollector<F> {
        BinaryAddHiCollector::new(self.collect_info[&chunk_id], self.std.clone())
    }
}

impl<F: PrimeField64> Instance<F> for BinaryAddHiInstance<F> {
    /// Computes the witness for the packed add execution plan.
    ///
    /// This method leverages the `BinaryAddHiSM` to generate an `AirInstance` using the collected
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
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let collector = collector.as_any().downcast::<BinaryAddHiCollector<F>>().unwrap();
                collector.inputs
            })
            .collect();

        // The two airs pack a different number of additions per row, so they have distinct row
        // types; the trace type selects both the row layout and the air the instance belongs to.
        match (self.air_id(), packed) {
            (AIR_ID, true) => Ok(Some(
                self.binary_add_hi_sm
                    .compute_witness::<_, BinaryAddHiTraceRowPacked<F>>(&inputs, trace_buffer)?,
            )),
            (AIR_ID, false) => Ok(Some(
                self.binary_add_hi_sm
                    .compute_witness::<_, BinaryAddHiTraceRow<F>>(&inputs, trace_buffer)?,
            )),
            (LARGE_AIR_ID, true) => Ok(Some(
                self.binary_add_hi_sm.compute_witness::<_, BinaryAddHiLargeTraceRowPacked<F>>(
                    &inputs,
                    trace_buffer,
                )?,
            )),
            (LARGE_AIR_ID, false) => Ok(Some(
                self.binary_add_hi_sm
                    .compute_witness::<_, BinaryAddHiLargeTraceRow<F>>(&inputs, trace_buffer)?,
            )),
            (HUGE_AIR_ID, true) => Ok(Some(
                self.binary_add_hi_sm.compute_witness::<_, BinaryAddHiHugeTraceRowPacked<F>>(
                    &inputs,
                    trace_buffer,
                )?,
            )),
            (HUGE_AIR_ID, false) => Ok(Some(
                self.binary_add_hi_sm
                    .compute_witness::<_, BinaryAddHiHugeTraceRow<F>>(&inputs, trace_buffer)?,
            )),
            (air_id, _) => panic!("BinaryAddHiInstance: Unsupported air_id: {air_id:?}"),
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
        Some(Box::new(BinaryAddHiCollector::new(self.collect_info[&chunk_id], self.std.clone())))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
