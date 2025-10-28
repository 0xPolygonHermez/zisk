use crate::{mem_align_byte_sm::MemAlignByteSM, MemAlignCollector};
use mem_common::MemAlignCheckPoint;

use crate::mem_align_byte_sm::{MemAlignWriteByteTraceRowType, MemAlignWriteByteTraceType};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::{collections::HashMap, sync::Arc};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};

pub struct MemAlignWriteByteInstance<F: PrimeField64> {
    /// Instance context
    ictx: InstanceCtx,

    /// Checkpoint data for this memory align instance.
    checkpoint: HashMap<ChunkId, MemAlignCheckPoint>,

    mem_align_byte_sm: Arc<MemAlignByteSM<F>>,
}

impl<F: PrimeField64> MemAlignWriteByteInstance<F> {
    pub fn new(mem_align_sm: Arc<MemAlignByteSM<F>>, mut ictx: InstanceCtx) -> Self {
        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let checkpoint = *meta
            .downcast::<HashMap<ChunkId, MemAlignCheckPoint>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { ictx, checkpoint, mem_align_byte_sm: mem_align_sm }
    }

    pub fn build_mem_align_write_byte_collector(&self, chunk_id: ChunkId) -> MemAlignCollector {
        MemAlignCollector::new(&self.checkpoint[&chunk_id])
    }
}

impl<F: PrimeField64> Instance<F> for MemAlignWriteByteInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> Option<AirInstance<F>> {
        let mut total_rows = 0;
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let collector = collector.as_any().downcast::<MemAlignCollector>().unwrap();

                total_rows += collector.count();

                collector.inputs
            })
            .collect();
        Some(
            self.mem_align_byte_sm
                .compute_witness::<MemAlignWriteByteTraceType<F>, MemAlignWriteByteTraceRowType<F>>(
                    &inputs,
                    total_rows as usize,
                    trace_buffer,
                ),
        )
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
}
