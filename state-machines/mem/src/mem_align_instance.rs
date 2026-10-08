use crate::{MemAlignCollector, MemAlignSM};
use zisk_sm_mem_common::MemAlignCheckPoint;

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{
    MemAlignLargeTrace, MemAlignTrace, MemAlignTraceRow, MemAlignTraceRowPacked, ZISK_AIRGROUP_ID,
};

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

    /// The instance the prover's kernel fills into its slot (`ZISK_MEM_GPU_FILL=slot`): the staged
    /// op and the air values the plan determines.
    fn slot_witness(
        &self,
        pctx: &ProofCtx<F>,
        air_id: usize,
        segment: usize,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        use proofman_common::trace::TraceRow;
        if !packed {
            return Err(proofman_common::ProofmanError::InvalidParameters(
                "ZISK_MEM_GPU_FILL=slot needs the packed MemAlign traces".to_string(),
            ));
        }
        let decl =
            pctx.gpu_witness_airs.get(self.ictx.plan.airgroup_id, air_id).ok_or_else(|| {
                proofman_common::ProofmanError::InvalidParameters(format!(
                    "ZISK_MEM_GPU_FILL=slot: air {air_id} has no GPU witness declaration"
                ))
            })?;
        let n_rows = zisk_sm_mem_common::mem_align_air_rows(air_id).ok_or_else(|| {
            proofman_common::ProofmanError::InvalidParameters(format!(
                "air {air_id} is not a MemAlign air"
            ))
        })?;
        let air_values = Vec::new();
        crate::mem_gpu_fill::slot_instance(
            decl,
            crate::mem_device_rows::SLOT_FAMILY_ALIGN,
            air_id,
            segment,
            n_rows,
            MemAlignTraceRow::<F>::ROW_SIZE,
            trace_buffer,
            air_values,
        )
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
                "ZISK_MEM_GPU_FILL=arena needs the packed MemAlign trace".to_string(),
            ));
        }
        if self.is_large() {
            Self::device_trace::<LARGE_ROWS, LARGE_AIR_ID>(air_id, segment, trace_buffer)
        } else {
            Self::device_trace::<ROWS, AIR_ID>(air_id, segment, trace_buffer)
        }
    }

    fn device_trace<const N: usize, const A: usize>(
        air_id: usize,
        segment: usize,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace =
            GenericTrace::<MemAlignTraceRowPacked<F>, N, ZISK_AIRGROUP_ID, A>::new_from_vec(
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
            MemAlignTraceRowPacked::<F>::PACKED_WORDS,
        );
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
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
        let air_id = self.ictx.plan.air_id;
        let segment =
            usize::from(self.ictx.plan.segment_id.expect("MemAlign plan without segment"));
        if crate::mem_device_rows::rows_on_device("align") {
            if crate::mem_device_rows::slot_pending("align") {
                return self.slot_witness(_pctx, air_id, segment, trace_buffer, packed).map(Some);
            }
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
        let sm = &self.mem_align_sm;
        let used_rows = total_rows as usize;
        Ok(Some(match (self.is_large(), packed) {
            (false, true) => sm.compute_witness::<MemAlignTraceRowPacked<F>, ROWS, AIR_ID>(
                &inputs,
                used_rows,
                trace_buffer,
                on_filled.take(),
            )?,
            (false, false) => sm.compute_witness::<MemAlignTraceRow<F>, ROWS, AIR_ID>(
                &inputs,
                used_rows,
                trace_buffer,
                on_filled.take(),
            )?,
            (true, true) => sm
                .compute_witness::<MemAlignTraceRowPacked<F>, LARGE_ROWS, LARGE_AIR_ID>(
                    &inputs,
                    used_rows,
                    trace_buffer,
                    on_filled.take(),
                )?,
            (true, false) => sm.compute_witness::<MemAlignTraceRow<F>, LARGE_ROWS, LARGE_AIR_ID>(
                &inputs,
                used_rows,
                trace_buffer,
                on_filled.take(),
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
