//! The instance of the fused `CompactDma` air.

use std::sync::Arc;

#[cfg(feature = "save_dma_collectors")]
use crate::save_dma_collectors;
use crate::{
    CompactDmaCollector, CompactDmaSM, DmaLoopCheckPoint, DmaLoopCollector,
    DmaWithPrePostCheckPoint, DmaWithPrePostCollector,
};
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType, StatsType,
};

/// What one `CompactDma` instance has to collect: the plan of each of its blocks.
///
/// The `loop_` block is a segment of its own continuation chain, whose segments are the instances
/// of this air, so its `is_last_segment` is the air's -- even when the block itself got nothing to
/// prove and is only padding.
#[derive(Default, Debug)]
pub struct CompactDmaCheckPoint {
    pub wpp: DmaWithPrePostCheckPoint,
    pub lp: DmaLoopCheckPoint,
}

impl CompactDmaCheckPoint {
    #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_plans"))]
    pub fn get_debug_info(&self, title: &str, segment_id: u64) -> String {
        self.wpp.get_debug_info(&format!("{title}.wpp"), segment_id)
            + "\n"
            + &self.lp.get_debug_info(&format!("{title}.loop"), segment_id)
    }
}

pub struct CompactDmaInstance<F: PrimeField64> {
    sm: Arc<CompactDmaSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> CompactDmaInstance<F> {
    pub fn new(sm: Arc<CompactDmaSM<F>>, ictx: InstanceCtx) -> Self {
        Self { sm, ictx }
    }

    fn check_point_data(&self) -> &CompactDmaCheckPoint {
        self.ictx.plan.meta.as_ref().unwrap().downcast_ref::<CompactDmaCheckPoint>().unwrap()
    }

    /// The collectors of one chunk: one per block whose plan reaches this chunk.
    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> CompactDmaCollector {
        let cp = self.check_point_data();
        let wpp = cp.wpp.chunks.get(&chunk_id).map(|&(num_ops, collect_counters)| {
            DmaWithPrePostCollector::new(chunk_id, num_ops, collect_counters)
        });
        let lp = cp.lp.chunks.get(&chunk_id).map(|&(num_inputs, collect_counters)| {
            DmaLoopCollector::new(chunk_id, num_inputs, collect_counters)
        });
        CompactDmaCollector::new(wpp, lp)
    }
}

impl<F: PrimeField64> Instance<F> for CompactDmaInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let mut wpp_inputs = Vec::new();
        let mut loop_inputs = Vec::new();
        #[cfg(feature = "save_dma_collectors")]
        let mut debug = Vec::new();
        for (_, collector) in collectors {
            let collector = collector.as_any().downcast::<CompactDmaCollector>().unwrap();
            #[cfg(feature = "save_dma_collectors")]
            debug.push(collector.get_debug_info());
            let CompactDmaCollector { wpp, lp } = *collector;
            if let Some(wpp) = wpp {
                wpp_inputs.push(wpp.inputs);
            }
            if let Some(mut lp) = lp {
                loop_inputs.push(lp.take_inputs());
            }
        }

        let segment_id = self.ictx.plan.segment_id.unwrap();

        #[cfg(feature = "save_dma_collectors")]
        save_dma_collectors(&format!("compact_dma_collector_{segment_id:04}.txt"), debug)?;

        let wpp_flat: Vec<_> = wpp_inputs.iter().flatten().collect();
        let loop_flat = crate::flatten_and_reorder_inputs(&loop_inputs);
        let is_last_segment = self.check_point_data().lp.is_last_segment;
        Ok(Some(self.sm.compute_witness(
            &wpp_flat,
            &loop_flat,
            segment_id,
            is_last_segment,
            trace_buffer,
            packed,
        )?))
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Precompiled
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(self.build_dma_collector(chunk_id)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
