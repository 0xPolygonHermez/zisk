use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{
    DmaTrace, DmaTraceRow, DmaTraceRowOps, DmaTraceRowPacked,
};
use std::marker::PhantomData;

use crate::{dma_trace, DmaInput, DmaModule};
use zisk_precomp_helpers::DmaInfo;

/// The `DmaSM` struct encapsulates the logic of the Dma State Machine.
pub struct DmaSM<F: PrimeField64> {
    _phantom: PhantomData<F>,
}

impl<F: PrimeField64> DmaSM<F> {
    /// Creates a new Dma State Machine instance.
    ///
    /// # Returns
    /// A new `DmaSM` instance.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            _phantom: PhantomData,
        })
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn process_slice<R: DmaTraceRowOps<F>>(
        &self,
        input: &DmaInput,
        // row_offset: usize,
        trace: &mut R,
    ) {
        let count = DmaInfo::get_count(input.encoded);
        let count_lt_256 = count < 256;
        let count_ge_256 = 1 - count_lt_256 as usize;
        let h_count = ((count >> 8) - count_ge_256) as u32;
        trace.set_count_lt_256(count_lt_256);
        trace.set_h_count(h_count);
        let l_count = (count & 0xFF) as u16 + 256 * count_ge_256 as u16;
        trace.set_l_count(l_count);

        let src = input.src as u32;
        let dst = input.dst as u32;
        let h_src64 = src >> 10;
        let h_dst64 = dst >> 10;
        let l_src64 = (src >> 3) as u8 & 0x7F;
        let l_dst64 = (dst >> 3) as u8 & 0x7F;

        trace.set_src_hi((input.src >> 32) as u32);
        trace.set_h_src64(h_src64);
        trace.set_l_src64(l_src64);
        let src_offset = src as u8 & 0x07;
        trace.set_src_offset(src_offset);

        trace.set_h_dst64(h_dst64);
        trace.set_l_dst64(l_dst64);
        trace.set_dst_hi((input.dst >> 32) as u32);
        trace.set_dst_offset(dst as u8 & 0x07);

        trace.set_main_step(input.step);

        let pre_count = DmaInfo::get_pre_count(input.encoded) as u8;
        let loop_count = DmaInfo::get_loop_count(input.encoded);
        let post_count = DmaInfo::get_post_count(input.encoded);
        trace.set_use_pre(pre_count > 0);
        trace.set_use_loop(loop_count > 0);
        trace.set_use_post(post_count > 0);

        trace.set_src64_inc_by_pre(DmaInfo::get_src64_inc_by_pre(input.encoded) > 0);

        trace.set_pre_count(pre_count);
        trace.set_l_count64((l_count - pre_count as u16 - post_count as u16) >> 3);

        let use_src = input.op != ZiskOp::DMA_INPUTCPY && input.op != ZiskOp::DMA_XMEMSET;
        if use_src {
            trace.set_src_offset_after_pre((src_offset + pre_count) % 8);
        }
        match input.op {
            ZiskOp::DMA_MEMCPY => trace.set_sel_memcpy(true),
            ZiskOp::DMA_XMEMCPY => {
                trace.set_sel_memcpy(true);
                trace.set_sel_extended(true);
            }
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
                trace.set_sel_memcmp(true);
                trace.set_sel_extended(input.op == ZiskOp::DMA_XMEMCMP);
                let pre_result_nz = DmaInfo::get_memcmp_pre_result_nz(input.encoded);
                let post_result_nz = DmaInfo::get_memcmp_post_result_nz(input.encoded);
                trace.set_pre_result_nz(pre_result_nz);
                trace.set_post_result_nz(post_result_nz);
                let count_diff = input.count_bus - count as u32;

                // INVALID ASSERT BECAUSE count_diff == 0 and diffent, case last byte is
                // different.
                // assert!(
                //     (count_diff == 0 && (pre_result_nz as u32 + post_result_nz as u32) == 0)
                //         || (count_diff != 0 && (pre_result_nz as u32 + post_result_nz as u32) == 1),
                //     "Invalid memcmp result for count_diff {count_diff}: ({}-{count}) \p
                //        pre_result_nz={pre_result_nz}, post_result_nz={post_result_nz} {}",
                //     input.count_bus,
                //     DmaInfo::to_string(input.encoded)
                // );

                let count_diff_chunks = [count_diff as u16, (count_diff >> 16) as u16];
                trace.set_all_count_diff_chunks(&count_diff_chunks);

                if pre_result_nz {
                    let result = DmaInfo::get_memcmp_res_as_u64(input.encoded);
                    let bus_pre_result = [result as u32, (result >> 32) as u32];
                    trace.set_all_bus_pre_result(&bus_pre_result);
                }
                if post_result_nz {
                    let result = DmaInfo::get_memcmp_res_as_u64(input.encoded);
                    let bus_post_result = [result as u32, (result >> 32) as u32];
                    trace.set_all_bus_post_result(&bus_post_result);
                }
            }
            ZiskOp::DMA_INPUTCPY => trace.set_sel_inputcpy(true),
            ZiskOp::DMA_XMEMSET => {
                trace.set_sel_memset(true);
                trace.set_sel_extended(true);
                trace.set_fill_byte(DmaInfo::get_fill_byte(input.encoded));
                // println!("XMEMSET fill_byte: 0x{:02X}", DmaInfo::get_fill_byte(input.encoded));
            }
            _ => panic!("Invalid DMA operation {}", input.op),
        }

    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_empty_slice<R: DmaTraceRowOps<F>>(&self, trace: &mut R) {
        // trace was initialized with zeroes
        trace.set_count_lt_256(true);
    }

    fn compute_witness_inner<R: DmaTraceRowOps<F> + Copy + Send>(
        &self,
        inputs: &[Vec<DmaInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        dma_trace("Dma", total_inputs, num_rows);

        timer_start_trace!(DMA_TRACE);

        // Split the dma_trace.buffer into slices matching each inner vector's length.
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect();
        let trace_rows = trace.buffer.as_mut_slice();
        // Calculate optimal chunk size
        let num_threads = rayon::current_num_threads();
        let chunk_size = std::cmp::max(1, flat_inputs.len() / num_threads);

        flat_inputs
            .par_chunks(chunk_size)
            .zip(trace_rows.par_chunks_mut(chunk_size))
            .for_each(|(input_chunk, trace_chunk)| {
                for (input, trace_row) in input_chunk.iter().zip(trace_chunk.iter_mut()) {
                    self.process_slice(input, trace_row);
                }
            });

        if total_inputs < num_rows {
            self.process_empty_slice(&mut trace_rows[total_inputs]);
            let empty_row = trace_rows[total_inputs];
            trace_rows[total_inputs + 1..].par_iter_mut().for_each(|row| {
                *row = empty_row;
            });
        }
        timer_stop_and_log_trace!(DMA_TRACE);
        let from_trace = FromTrace::new(&mut trace);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}
impl<F: PrimeField64> DmaModule<F> for DmaSM<F> {
    fn get_name(&self) -> &'static str {
        "dma"
    }
    fn compute_witness(
        &self,
        inputs: &[Vec<DmaInput>],
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<DmaTraceRowPacked<F>>(inputs, trace_buffer)
        } else {
            self.compute_witness_inner::<DmaTraceRow<F>>(inputs, trace_buffer)
        }
    }
}
