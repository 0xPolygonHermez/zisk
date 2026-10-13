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

const ROWS: usize = MemAlignWriteByteTrace::<()>::NUM_ROWS;
const AIR_ID: usize = MemAlignWriteByteTrace::<()>::AIR_ID;

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

    /// The instance from the rows the GPU planner built (`ZISK_MEM_GPU_FILL`).
    fn device_witness(
        &self,
        air_id: usize,
        segment: usize,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        use crate::mem_device_rows::align_device_instance;
        let words = MemAlignWriteByteTraceRowPacked::<F>::PACKED_WORDS;
        align_device_instance::<F, MemAlignWriteByteTraceRowPacked<F>, ROWS, AIR_ID>(
            air_id,
            segment,
            trace_buffer,
            packed,
            words,
            |t, used| {
                let mut v = MemAlignWriteByteAirValues::<F>::new();
                v.padding_size = F::from_usize(t.num_rows() - used);
                AirInstance::new_from_trace(FromTrace::new(t).with_air_values(&mut v))
            },
        )
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
        // No collectors: the executor left the rows to the device (a device failure here makes
        // it collect the instance and come back with collectors for the CPU witness).
        if collectors.is_empty() {
            return self.device_witness(air_id, segment, trace_buffer, packed).map(Some);
        }
        let mut total_rows = 0;
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let collector = collector.as_any().downcast::<MemAlignCollector>().unwrap();

                total_rows += collector.count();

                collector.inputs
            })
            .collect();
        let instance = if packed {
            self.mem_align_byte_sm
                .compute_witness::<
                    MemAlignWriteByteTrace<MemAlignWriteByteTraceRowPacked<F>>,
                    MemAlignWriteByteTraceRowPacked<F>,
                >(&inputs, total_rows as usize, trace_buffer)?
        } else {
            self.mem_align_byte_sm
                .compute_witness::<
                    MemAlignWriteByteTrace<MemAlignWriteByteTraceRow<F>>,
                    MemAlignWriteByteTraceRow<F>,
                >(&inputs, total_rows as usize, trace_buffer)?
        };
        if packed {
            let words = MemAlignWriteByteTraceRowPacked::<F>::PACKED_WORDS;
            crate::mem_device_rows::align_dump(
                air_id,
                segment,
                &crate::mem_trace_hash::rows_as_words(&instance.trace)[..instance.num_rows * words],
                words,
            );
        }
        Ok(Some(instance))
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
