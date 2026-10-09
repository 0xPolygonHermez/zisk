//! The instance of the `DmaLoop` air.

#[cfg(feature = "save_dma_collectors")]
use crate::save_dma_collectors;
#[cfg(feature = "save_dma_inputs")]
use crate::DmaLoopInput;
use crate::{DmaLoopCheckPoint, DmaLoopCollector, DmaLoopSM};
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::sync::Arc;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType, StatsType,
};
use zisk_pil::DmaLoopTrace;

pub struct DmaLoopInstance<F: PrimeField64> {
    sm: Arc<DmaLoopSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> DmaLoopInstance<F> {
    pub fn new(sm: Arc<DmaLoopSM<F>>, ictx: InstanceCtx) -> Self {
        Self { sm, ictx }
    }

    fn check_point_data(&self) -> &DmaLoopCheckPoint {
        self.ictx.plan.meta.as_ref().unwrap().downcast_ref::<DmaLoopCheckPoint>().unwrap()
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> DmaLoopCollector {
        debug_assert_eq!(
            self.ictx.plan.air_id,
            DmaLoopTrace::<()>::AIR_ID,
            "DmaLoopInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );
        let (num_inputs, collect_counters) = self.check_point_data().chunks[&chunk_id];
        DmaLoopCollector::new(chunk_id, num_inputs, collect_counters)
    }
}

impl<F: PrimeField64> Instance<F> for DmaLoopInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        #[cfg(feature = "save_dma_collectors")]
        let (debug, inputs): (Vec<_>, Vec<_>) = collectors
            .into_iter()
            .map(|(_, collector)| {
                let mut collector = collector.as_any().downcast::<DmaLoopCollector>().unwrap();
                (collector.get_debug_info(), collector.take_inputs())
            })
            .unzip();
        #[cfg(not(feature = "save_dma_collectors"))]
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<DmaLoopCollector>().unwrap().take_inputs()
            })
            .collect();

        let segment_id = self.ictx.plan.segment_id.unwrap();

        #[cfg(feature = "save_dma_collectors")]
        save_dma_collectors(&format!("dma_loop_collector_{segment_id:04}.txt"), debug)?;

        #[cfg(feature = "save_dma_inputs")]
        DmaLoopInput::dump_to_file(&inputs, &format!("dma_loop_inputs_{segment_id:04}.txt"))?;

        let is_last_segment = self.check_point_data().is_last_segment;
        Ok(Some(self.sm.compute_witness(
            &inputs,
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
