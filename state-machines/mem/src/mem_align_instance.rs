use crate::{MemAlignCollector, MemAlignSM};
use zisk_sm_mem_common::MemAlignCheckPoint;

use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{MemAlignLargeTrace, MemAlignTrace, MemAlignTraceRow, MemAlignTraceRowPacked};

/// Height and air id of each `MemAlign` air, as const-generic arguments for the witness computation.
const ROWS: usize = MemAlignTrace::<()>::NUM_ROWS;
const AIR_ID: usize = MemAlignTrace::<()>::AIR_ID;
const LARGE_ROWS: usize = MemAlignLargeTrace::<()>::NUM_ROWS;
const LARGE_AIR_ID: usize = MemAlignLargeTrace::<()>::AIR_ID;

pub struct MemAlignInstance<F: PrimeField64> {
    /// Instance context
    ictx: InstanceCtx,

    /// Checkpoint data for this memory align instance.
    checkpoint: HashMap<ChunkId, MemAlignCheckPoint>,

    mem_align_sm: Arc<MemAlignSM<F>>,
}

impl<F: PrimeField64> MemAlignInstance<F> {
    pub fn new(mem_align_sm: Arc<MemAlignSM<F>>, mut ictx: InstanceCtx) -> Self {
        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let checkpoint = *meta
            .downcast::<HashMap<ChunkId, MemAlignCheckPoint>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { ictx, checkpoint, mem_align_sm }
    }

    /// `true` when this instance is the tall air. The two commit the same columns, so this only
    /// picks the height and air id the trace is built with.
    fn is_large(&self) -> bool {
        self.ictx.plan.air_id == LARGE_AIR_ID
    }

    pub fn build_mem_align_collector(&self, chunk_id: ChunkId) -> MemAlignCollector {
        MemAlignCollector::new(&self.checkpoint[&chunk_id])
    }
}

impl<F: PrimeField64> Instance<F> for MemAlignInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let mut total_rows = 0;
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let collector = collector.as_any().downcast::<MemAlignCollector>().unwrap();

                total_rows += collector.count();

                collector.inputs
            })
            .collect();
        let sm = &self.mem_align_sm;
        let used_rows = total_rows as usize;
        Ok(Some(match (self.is_large(), packed) {
            (false, true) => sm.compute_witness::<MemAlignTraceRowPacked<F>, ROWS, AIR_ID>(
                &inputs,
                used_rows,
                trace_buffer,
            )?,
            (false, false) => sm.compute_witness::<MemAlignTraceRow<F>, ROWS, AIR_ID>(
                &inputs,
                used_rows,
                trace_buffer,
            )?,
            (true, true) => sm
                .compute_witness::<MemAlignTraceRowPacked<F>, LARGE_ROWS, LARGE_AIR_ID>(
                    &inputs,
                    used_rows,
                    trace_buffer,
                )?,
            (true, false) => sm.compute_witness::<MemAlignTraceRow<F>, LARGE_ROWS, LARGE_AIR_ID>(
                &inputs,
                used_rows,
                trace_buffer,
            )?,
        }))
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Memory
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(MemAlignCollector::new(&self.checkpoint[&chunk_id])))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
