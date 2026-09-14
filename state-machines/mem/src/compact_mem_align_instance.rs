use crate::{CompactMemAlignSM, MemAlignCollector};
use zisk_sm_mem_common::MemAlignCheckPoint;

use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::CompactMemAlignLargeTrace;

/// One instance of the fused air, which proves the two kinds of unaligned access at once.
///
/// Its checkpoint is the very one the standalone instances take -- one `MemAlignCheckPoint` per
/// chunk, carrying the five kinds of operation -- because the planner merges the two blocks'
/// checkpoints into it: the `full_*` counters come from the block that proves them and the byte
/// ones from the other. That is what lets the plain `MemAlignCollector` feed this instance.
pub struct CompactMemAlignInstance<F: PrimeField64> {
    /// Instance context
    ictx: InstanceCtx,

    /// Checkpoint data for this instance.
    checkpoint: HashMap<ChunkId, MemAlignCheckPoint>,

    compact_mem_align_sm: Arc<CompactMemAlignSM<F>>,
}

impl<F: PrimeField64> CompactMemAlignInstance<F> {
    pub fn new(compact_mem_align_sm: Arc<CompactMemAlignSM<F>>, mut ictx: InstanceCtx) -> Self {
        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let checkpoint = *meta
            .downcast::<HashMap<ChunkId, MemAlignCheckPoint>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { ictx, checkpoint, compact_mem_align_sm }
    }

    /// `true` when this instance is the tall air. The two commit the same columns, so this only
    /// picks the height and air id the trace is built with.
    fn is_large(&self) -> bool {
        self.ictx.plan.air_id == CompactMemAlignLargeTrace::<()>::AIR_ID
    }

    pub fn build_compact_mem_align_collector(&self, chunk_id: ChunkId) -> MemAlignCollector {
        MemAlignCollector::new(&self.checkpoint[&chunk_id])
    }
}

impl<F: PrimeField64> Instance<F> for CompactMemAlignInstance<F> {
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
                let collector = collector.as_any().downcast::<MemAlignCollector>().unwrap();
                collector.inputs
            })
            .collect();

        Ok(Some(self.compact_mem_align_sm.compute_witness(
            &inputs,
            self.is_large(),
            packed,
            trace_buffer,
        )?))
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

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(MemAlignCollector::new(&self.checkpoint[&chunk_id])))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
