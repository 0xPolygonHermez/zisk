//! The `BinaryAddInstance` module defines an specific instance to perform witness computations
//! for binary add operations using the Binary Add State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryAddSM` to compute witnesses for
//! execution plans.

use crate::{BinaryAddCollector, BinaryAddSM, ChunkCollect, ADD_KINDS};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{
    BinaryAddHugeTrace, BinaryAddHugeTraceRow, BinaryAddHugeTraceRowPacked, BinaryAddLargeTrace,
    BinaryAddLargeTraceRow, BinaryAddLargeTraceRowPacked, BinaryAddTrace, BinaryAddTraceRow,
    BinaryAddTraceRowPacked,
};

/// Air id of each `BinaryAdd` air. They no longer differ only in height: each packs a different
/// number of operations per row, so each has its own row type.
const AIR_ID: usize = BinaryAddTrace::<()>::AIR_ID;
const LARGE_AIR_ID: usize = BinaryAddLargeTrace::<()>::AIR_ID;
const HUGE_AIR_ID: usize = BinaryAddHugeTrace::<()>::AIR_ID;

/// The `BinaryAddInstance` struct represents an instance for binary add witness computations.
///
/// It encapsulates the `BinaryAddSM` and its associated context, and it processes input data
/// to compute witnesses for binary operations.
pub struct BinaryAddInstance<F: PrimeField64> {
    /// Binary Add state machine.
    binary_add_sm: Arc<BinaryAddSM<F>>,

    /// What this instance takes from each chunk: a `(count, skip)` per kind of operation, plus the
    /// frequent operations it accounts for.
    collect_info: HashMap<ChunkId, ChunkCollect<ADD_KINDS>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
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
    pub fn new(
        binary_add_sm: Arc<BinaryAddSM<F>>,
        mut ictx: InstanceCtx,
        std: Arc<Std<F>>,
    ) -> Self {
        assert!(
            matches!(ictx.plan.air_id, AIR_ID | LARGE_AIR_ID | HUGE_AIR_ID),
            "BinaryAddInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, ChunkCollect<ADD_KINDS>>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { binary_add_sm, collect_info, ictx, std }
    }

    /// Which of the three `BinaryAdd` airs this instance is. They pack a different number of
    /// operations per row, so this picks the row type the trace is built with.
    fn air_id(&self) -> usize {
        self.ictx.plan.air_id
    }

    pub fn build_binary_add_collector(&self, chunk_id: ChunkId) -> BinaryAddCollector<F> {
        BinaryAddCollector::new(self.collect_info[&chunk_id], self.std.clone())
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
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let _collector = collector.as_any().downcast::<BinaryAddCollector<F>>().unwrap();
                _collector.inputs
            })
            .collect();

        let sm = &self.binary_add_sm;
        Ok(Some(match (self.air_id(), packed) {
            (AIR_ID, true) => {
                sm.compute_witness::<_, BinaryAddTraceRowPacked<F>>(&inputs, trace_buffer)?
            }
            (AIR_ID, false) => {
                sm.compute_witness::<_, BinaryAddTraceRow<F>>(&inputs, trace_buffer)?
            }
            (LARGE_AIR_ID, true) => {
                sm.compute_witness::<_, BinaryAddLargeTraceRowPacked<F>>(&inputs, trace_buffer)?
            }
            (LARGE_AIR_ID, false) => {
                sm.compute_witness::<_, BinaryAddLargeTraceRow<F>>(&inputs, trace_buffer)?
            }
            (HUGE_AIR_ID, true) => {
                sm.compute_witness::<_, BinaryAddHugeTraceRowPacked<F>>(&inputs, trace_buffer)?
            }
            (HUGE_AIR_ID, false) => {
                sm.compute_witness::<_, BinaryAddHugeTraceRow<F>>(&inputs, trace_buffer)?
            }
            (air_id, _) => panic!("BinaryAddInstance: Unsupported air_id: {air_id:?}"),
        }))
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
        assert_eq!(
            self.ictx.plan.air_id,
            BinaryAddTrace::<()>::AIR_ID,
            "BinaryAddInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );
        Some(Box::new(BinaryAddCollector::new(self.collect_info[&chunk_id], self.std.clone())))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
