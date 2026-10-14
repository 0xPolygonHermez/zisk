//! The `DmaCompactInstance` performs the witness computation of one `DmaCompact` instance from
//! the inputs its collectors gathered.

use std::sync::Arc;

use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType, StatsType,
};
use zisk_pil::DmaCompactTrace;

use crate::{DmaCompactCheckPoint, DmaCompactCollector, DmaCompactSM};

pub struct DmaCompactInstance<F: PrimeField64> {
    /// DmaCompact state machine.
    sm: Arc<DmaCompactSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> DmaCompactInstance<F> {
    pub fn new(sm: Arc<DmaCompactSM<F>>, ictx: InstanceCtx) -> Self {
        debug_assert_eq!(
            ictx.plan.air_id,
            DmaCompactTrace::<()>::AIR_ID,
            "DmaCompactInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );
        Self { sm, ictx }
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> DmaCompactCollector {
        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let check_point: &DmaCompactCheckPoint = meta.downcast_ref().unwrap();
        DmaCompactCollector::new(chunk_id, check_point.chunks[&chunk_id])
    }
}

impl<F: PrimeField64> Instance<F> for DmaCompactInstance<F> {
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
                let collector = collector.as_any().downcast::<DmaCompactCollector>().unwrap();
                (collector.get_debug_info(), collector.inputs)
            })
            .unzip();
        #[cfg(not(feature = "save_dma_collectors"))]
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<DmaCompactCollector>().unwrap().inputs
            })
            .collect();

        #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_inputs"))]
        let air_instance_id =
            _pctx.dctx_find_air_instance_id(self.ictx.plan.global_id.unwrap()).unwrap();

        #[cfg(feature = "save_dma_collectors")]
        crate::save_dma_collectors(
            &format!("dma_compact_collector_{air_instance_id:04}.txt"),
            debug,
        )?;

        #[cfg(feature = "save_dma_inputs")]
        crate::DmaCompactInput::dump_to_file(
            &inputs,
            &format!("dma_compact_inputs_{air_instance_id:04}.txt"),
        )?;

        Ok(Some(self.sm.compute_witness(&inputs, trace_buffer, packed)?))
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
