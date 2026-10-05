use crate::{mem_align_byte_sm::MemAlignByteSM, MemAlignCollector};
use zisk_sm_mem_common::MemAlignCheckPoint;

use proofman_common::{AirInstance, FromTrace, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{
    MemAlignWriteByteAirValues, MemAlignWriteByteTrace, MemAlignWriteByteTraceRow,
    MemAlignWriteByteTraceRowPacked,
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

    /// The instance from the rows the GPU planner built (`ZISK_MEM_GPU_FILL=arena`).
    fn device_witness(
        &self,
        air_id: usize,
        segment: usize,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if !packed {
            return Err(proofman_common::ProofmanError::InvalidParameters(
                "ZISK_MEM_GPU_FILL=arena needs the packed MemAlign byte trace".to_string(),
            ));
        }
        let mut trace = MemAlignWriteByteTrace::<MemAlignWriteByteTraceRowPacked<F>>::new_from_vec(
            trace_buffer,
        )?;
        let used = crate::mem_device_rows::align_device_rows(
            air_id,
            segment,
            crate::mem_trace_hash::rows_as_words_mut(&mut trace.buffer),
        )?;
        crate::mem_device_rows::align_dump(
            air_id,
            segment,
            crate::mem_trace_hash::rows_as_words(&trace.buffer),
            used,
            MemAlignWriteByteTraceRowPacked::<F>::PACKED_WORDS,
        );
        let mut air_values = MemAlignWriteByteAirValues::<F>::new();
        air_values.padding_size = F::from_usize(trace.num_rows() - used);
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }
}

impl<F: PrimeField64> Instance<F> for MemAlignWriteByteInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let air_id = self.ictx.plan.air_id;
        let segment =
            usize::from(self.ictx.plan.segment_id.expect("MemAlign plan without segment"));
        if crate::mem_device_rows::rows_on_device("align") {
            return self.device_witness(air_id, segment, trace_buffer, packed).map(Some);
        }
        let mut hook = move |cpu: &[u64], f: &crate::mem_gpu_fill::RowsFilled| {
            crate::mem_device_rows::align_filled(air_id, segment, cpu, f.used, f.words_per_row);
        };
        let mut on_filled: crate::mem_gpu_fill::OnFilled<'_, crate::mem_gpu_fill::RowsFilled> =
            if packed { Some(&mut hook) } else { None };
        let mut total_rows = 0;
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let collector = collector.as_any().downcast::<MemAlignCollector>().unwrap();

                total_rows += collector.count();

                collector.inputs
            })
            .collect();
        Ok(Some(if packed {
            self.mem_align_byte_sm
                .compute_witness::<
                    MemAlignWriteByteTrace<MemAlignWriteByteTraceRowPacked<F>>,
                    MemAlignWriteByteTraceRowPacked<F>,
                >(&inputs, total_rows as usize, trace_buffer, on_filled.take())?
        } else {
            self.mem_align_byte_sm
                .compute_witness::<
                    MemAlignWriteByteTrace<MemAlignWriteByteTraceRow<F>>,
                    MemAlignWriteByteTraceRow<F>,
                >(&inputs, total_rows as usize, trace_buffer, on_filled.take())?
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
