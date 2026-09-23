//! Witness of the fused `CompactDma` air: a `DmaWithPrePost` block and a `DmaLoop` block, on the
//! same rows.

use std::sync::Arc;

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::SegmentId;
use zisk_pil::{
    CompactDmaAirValues, CompactDmaTrace, CompactDmaTraceRow, CompactDmaTraceRowPacked,
};

use crate::{
    dma_trace, DmaLoopBlockRow, DmaLoopInput, DmaLoopSM, DmaWithPrePostBlockRow,
    DmaWithPrePostInput, DmaWithPrePostSM,
};

/// Fills the `CompactDma` trace by running the two fills over the same rows.
///
/// There is no proving logic of its own here: each block is filled by the state machine of the air
/// it comes from, through the row adapters of [`crate::dma_block_rows`], and each fill only ever
/// touches its own columns. What this type adds is the one thing the two cannot do separately --
/// putting their results in a single `AirInstance`.
pub struct CompactDmaSM<F: PrimeField64> {
    wpp_sm: Arc<DmaWithPrePostSM<F>>,
    loop_sm: Arc<DmaLoopSM<F>>,
}

impl<F: PrimeField64> CompactDmaSM<F> {
    /// Shares the two state machines with the standalone instances rather than building its own:
    /// they hold the `Std` range-check ids, and the ids must be the same ones the standalone airs
    /// raise their checks against.
    pub fn new(wpp_sm: Arc<DmaWithPrePostSM<F>>, loop_sm: Arc<DmaLoopSM<F>>) -> Arc<Self> {
        Arc::new(Self { wpp_sm, loop_sm })
    }

    /// Computes the witness of one `CompactDma` instance.
    ///
    /// `loop_inputs` must come as [`crate::flatten_and_reorder_inputs`] gives them, and
    /// `loop_segment_id` / `loop_is_last_segment` place this instance in the chain of its loop
    /// block, whose segments are the instances of this air.
    pub fn compute_witness(
        &self,
        wpp_inputs: &[&DmaWithPrePostInput],
        loop_inputs: &[&DmaLoopInput],
        loop_segment_id: SegmentId,
        loop_is_last_segment: bool,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<CompactDmaTraceRowPacked<F>>(
                wpp_inputs,
                loop_inputs,
                loop_segment_id,
                loop_is_last_segment,
                trace_buffer,
            )
        } else {
            self.compute_witness_inner::<CompactDmaTraceRow<F>>(
                wpp_inputs,
                loop_inputs,
                loop_segment_id,
                loop_is_last_segment,
                trace_buffer,
            )
        }
    }

    fn compute_witness_inner<R>(
        &self,
        wpp_inputs: &[&DmaWithPrePostInput],
        loop_inputs: &[&DmaLoopInput],
        loop_segment_id: SegmentId,
        loop_is_last_segment: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>>
    where
        R: DmaWithPrePostBlockRow<F> + DmaLoopBlockRow<F>,
    {
        // Zeroed: the `wpp_` fill leaves the rows past its operations as the buffer holds them,
        // and a zero row is that block's padding. The `loop_` fill writes every row of its block.
        let mut trace = CompactDmaTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        let wpp_rows: usize = wpp_inputs.iter().map(|input| input.rows()).sum();
        let loop_rows: usize = loop_inputs.iter().map(|input| input.rows as usize).sum();
        dma_trace("CompactDma", wpp_rows.max(loop_rows), num_rows);

        timer_start_trace!(COMPACT_DMA_TRACE);
        // The two fills write disjoint columns of the same rows, so the order does not matter.
        let wpp_mults = DmaWithPrePostSM::<F>::fill_rows(wpp_inputs, trace.buffer.as_mut_slice());
        let loop_fill = DmaLoopSM::<F>::fill_rows(
            loop_inputs,
            trace.buffer.as_mut_slice(),
            loop_segment_id,
            loop_is_last_segment,
        );
        self.wpp_sm.charge(wpp_mults);
        self.loop_sm.charge(&loop_fill);
        timer_stop_and_log_trace!(COMPACT_DMA_TRACE);

        // The `wpp_` block has no air values; the `loop_` block carries the ones of `DmaLoop`
        // under its prefix.
        let mut air_values = CompactDmaAirValues::<F>::new();
        let v = &loop_fill.values;
        air_values.loop_segment_id = F::from_usize(v.segment_id);
        air_values.loop_is_last_segment = F::from_bool(v.is_last_segment);
        air_values.loop_padding_size = F::from_usize(v.padding_size);
        air_values.loop_segment_previous_seq_end = F::from_bool(v.previous.seq_end);
        air_values.loop_segment_previous_src64 = F::from_u32(v.previous.src64);
        air_values.loop_segment_previous_dst64 = F::from_u32(v.previous.dst64);
        air_values.loop_segment_previous_main_step = F::from_u64(v.previous.main_step);
        air_values.loop_segment_previous_count = F::from_u32(v.previous.count);
        air_values.loop_segment_previous_flags = F::from_u64(v.previous.flags);
        air_values.loop_segment_previous_fill_byte = F::from_u8(v.previous.fill_byte);
        air_values.loop_segment_first_bytes = v.previous.bytes.map(F::from_u8);
        air_values.loop_segment_last_seq_end = F::from_bool(v.last.seq_end);
        air_values.loop_segment_last_src64 = F::from_u32(v.last.src64);
        air_values.loop_segment_last_dst64 = F::from_u32(v.last.dst64);
        air_values.loop_segment_last_main_step = F::from_u64(v.last.main_step);
        air_values.loop_segment_last_count = F::from_u32(v.last.count);
        air_values.loop_segment_last_flags = F::from_u64(v.last.flags);
        air_values.loop_segment_last_fill_byte = F::from_u8(v.last.fill_byte);
        air_values.loop_segment_next_bytes = v.last.bytes.map(F::from_u8);
        air_values.loop_last_count_chunk = v.last_count_chunks().map(F::from_u16);

        let from_trace = FromTrace::new(&mut trace).with_air_values(&mut air_values);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}

#[cfg(test)]
#[path = "../tests/compact_dma_witness_tests.rs"]
mod tests;
