//! The `DmaWithPrePostInstance` module defines an instance to perform the witness computation
//! for the fused DmaWithPrePost State Machine.

#[cfg(feature = "save_dma_collectors")]
use crate::save_dma_collectors;
#[cfg(feature = "save_dma_inputs")]
use crate::DmaWithPrePostInput;
use crate::{DmaWithPrePostCheckPoint, DmaWithPrePostCollector, DmaWithPrePostModule};
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::sync::Arc;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType, StatsType,
};
use zisk_pil::DmaWithPrePostTrace;

/// The `DmaWithPrePostInstance` struct represents an instance for the fused DmaWithPrePost State
/// Machine.
pub struct DmaWithPrePostInstance<F: PrimeField64> {
    /// DmaWithPrePost state machine.
    module: Arc<dyn DmaWithPrePostModule<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> DmaWithPrePostInstance<F> {
    pub fn new(module: Arc<dyn DmaWithPrePostModule<F>>, ictx: InstanceCtx) -> Self {
        Self { module, ictx }
    }

    fn new_collector(&self, chunk_id: ChunkId) -> DmaWithPrePostCollector {
        debug_assert_eq!(
            self.ictx.plan.air_id,
            DmaWithPrePostTrace::<()>::AIR_ID,
            "DmaWithPrePostInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaWithPrePostCheckPoint>().unwrap();
        let (num_ops, collect_counters) = collect_info.chunks[&chunk_id];
        DmaWithPrePostCollector::new(chunk_id, num_ops, collect_counters)
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> DmaWithPrePostCollector {
        self.new_collector(chunk_id)
    }
}

impl<F: PrimeField64> Instance<F> for DmaWithPrePostInstance<F> {
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
                let collector = collector.as_any().downcast::<DmaWithPrePostCollector>().unwrap();
                (collector.get_debug_info(), collector.inputs)
            })
            .unzip();
        #[cfg(not(feature = "save_dma_collectors"))]
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<DmaWithPrePostCollector>().unwrap().inputs
            })
            .collect();

        #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_inputs"))]
        let air_instance_id =
            _pctx.dctx_find_air_instance_id(self.ictx.plan.global_id.unwrap()).unwrap();

        #[cfg(feature = "save_dma_collectors")]
        save_dma_collectors(
            &format!("{}_collector_{air_instance_id:04}.txt", self.module.get_name()),
            debug,
        )?;

        #[cfg(feature = "save_dma_inputs")]
        DmaWithPrePostInput::dump_to_file(
            &inputs,
            &format!("{}_inputs_{air_instance_id:04}.txt", self.module.get_name()),
        )?;

        Ok(Some(self.module.compute_witness(&inputs, trace_buffer, packed)?))
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
        Some(Box::new(self.new_collector(chunk_id)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
