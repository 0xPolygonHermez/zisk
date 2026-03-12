use std::sync::Arc;

use fields::PrimeField64;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::SegmentId;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::Dma64AlignedMemCpyAirValues;

#[cfg(feature = "packed")]
pub use zisk_pil::{
    Dma64AlignedMemCpyTracePacked as Dma64AlignedMemCpyTrace,
    Dma64AlignedMemCpyTraceRowPacked as Dma64AlignedMemCpyTraceRow,
};

#[cfg(not(feature = "packed"))]
pub use zisk_pil::{Dma64AlignedMemCpyTrace, Dma64AlignedMemCpyTraceRow};

use crate::{
    dma_trace, Dma64AlignedInput, Dma64AlignedModule, DMA_64_ALIGNED_MEMCPY_OPS_BY_ROW,
    F_SEL_MEMCPY,
};
use precompiles_helpers::DmaInfo;

/// The `Dma64AlignedMemCpySM` struct encapsulates the logic of the Dma64Aligned State Machine.
pub struct Dma64AlignedMemCpySM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Range checks ID's
    range_16_bits_id: usize,
    op_x_rows: usize,
}

impl<F: PrimeField64> Dma64AlignedMemCpySM<F> {
    /// Creates a new Dma State Machine instance.
    ///
    /// # Returns
    /// A new `Dma64AlignedMemCpySM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            range_16_bits_id: std
                .get_range_id(0, 0xFFFF, None)
                .expect("Failed to get 16b table ID"),
            op_x_rows: DMA_64_ALIGNED_MEMCPY_OPS_BY_ROW,
        })
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_input(
        &self,
        input: &Dma64AlignedInput,
        trace: &mut [Dma64AlignedMemCpyTraceRow<F>],
        _local_16_bits_table: &mut [u32], // for input_cpy
        air_values: &mut Dma64AlignedMemCpyAirValues<F>,
    ) -> usize {
        let rows = input.rows as usize;
        let is_last_instance_input = rows >= trace.len();
        let skip_count = input.skip_rows as usize * self.op_x_rows;
        let initial_count = DmaInfo::get_loop_count(input.encoded) - skip_count;
        let mut count64 = initial_count;

        let mut src_values_index = 0;
        let mut dst64 = ((input.dst + 7) >> 3) + skip_count as u32;
        let mut src64 = ((input.src + 7) >> 3) + skip_count as u32;
        let mut seq_end = false;
        let addr_incr_by_row = self.op_x_rows as u32;
        for (irow, row) in trace.iter_mut().enumerate().take(rows) {
            row.set_main_step(input.step);
            row.set_sel_memcpy(input.op == ZiskOp::DMA_XMEMCPY || input.op == ZiskOp::DMA_MEMCPY);
            row.set_previous_seq_end(irow == 0 && input.skip_rows == 0);

            if irow == 0 && input.skip_rows == 0 {
                row.set_sel_memcpy_count_load(input.op == ZiskOp::DMA_MEMCPY);
            }

            // calculate the first aligned address
            // if dst is aligned is same address if not it's addr + 8
            row.set_dst64(dst64);
            row.set_src64(src64);
            dst64 += addr_incr_by_row;
            src64 += addr_incr_by_row;

            row.set_count64(count64 as u32);
            let use_count = if count64 <= self.op_x_rows {
                seq_end = true;
                // trace i zerofilled, not set values zero
                count64
            } else {
                count64 -= self.op_x_rows;
                self.op_x_rows
            };
            row.set_seq_end(seq_end);
            for index in 0..use_count {
                if index > 0 {
                    row.set_sel_op_from_1(index - 1, true);
                }
                let value = input.src_values[src_values_index];
                src_values_index += 1;
                row.set_value(index, 0, value as u32);
                row.set_value(index, 1, (value >> 32) as u32);
            }
        }

        if is_last_instance_input {
            if seq_end {
                air_values.segment_last_seq_end = F::ONE;
                air_values.segment_last_src64 = F::ZERO;
                air_values.segment_last_dst64 = F::ZERO;
                air_values.segment_last_main_step = F::ZERO;
                air_values.segment_last_count64 = F::ZERO;
                air_values.last_count_chunk[0] = F::ZERO;
                air_values.last_count_chunk[1] = F::ZERO;
                air_values.segment_last_flags = F::ZERO;
            } else {
                air_values.segment_last_seq_end = F::ZERO;
                air_values.segment_last_src64 = F::from_u32(src64 - addr_incr_by_row);
                air_values.segment_last_dst64 = F::from_u32(dst64 - addr_incr_by_row);
                air_values.segment_last_main_step = F::from_u64(input.step);
                let last_count = initial_count - (rows - 1) * self.op_x_rows;
                air_values.segment_last_count64 = F::from_u32(last_count as u32);
                air_values.last_count_chunk[0] = F::from_u16(last_count as u16);
                air_values.last_count_chunk[1] = F::from_u16((last_count >> 16) as u16);
                air_values.segment_last_flags = F::from_u16(F_SEL_MEMCPY as u16);
            }
        }
        rows
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_empty_slice(&self, trace: &mut Dma64AlignedMemCpyTraceRow<F>) {
        trace.set_seq_end(true);
        trace.set_previous_seq_end(true);
    }
}
impl<F: PrimeField64> Dma64AlignedModule<F> for Dma64AlignedMemCpySM<F> {
    fn get_name(&self) -> &'static str {
        "dma_64_aligned_memcpy"
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `sctx` - The setup context containing the setup data.
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    fn compute_witness(
        &self,
        inputs: &[Vec<Dma64AlignedInput>],
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = Dma64AlignedMemCpyTrace::<F>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs
            .iter()
            .map(|inputs| inputs.iter().map(|input| input.rows as usize).sum::<usize>())
            .sum();

        assert!(total_inputs > 0);
        // println!("LAST INPUT: {:?}", inputs.last().unwrap());
        // println!("DMA_64_ALIGNED TOTALS total_inputs:{total_inputs} num_rows:{num_rows}");
        assert!(
            total_inputs <= num_rows,
            "Too many inputs, total_inputs:{total_inputs} num_rows:{num_rows}"
        );

        dma_trace("Dma64AlignedMemCpy", total_inputs, num_rows);

        timer_start_trace!(DMA_64_ALIGNED_TRACE);

        // Flat the inputs and reorder to ensure first, last are in theirs positions.
        let flat_inputs = crate::flatten_and_reorder_inputs(inputs);
        let trace_rows = trace.buffer.as_mut_slice();

        let mut local_16_bits_table = vec![0u32; 1 << 16];
        let mut air_values = Dma64AlignedMemCpyAirValues::<F>::new();

        // TODO: inputs between instances
        let mut row_offset = 0;
        for input in flat_inputs.iter() {
            let rows_used = self.process_input(
                input,
                &mut trace_rows[row_offset..],
                &mut local_16_bits_table,
                &mut air_values,
            );
            row_offset += rows_used;
        }

        // padding
        let padding_size = num_rows.saturating_sub(row_offset);
        air_values.padding_size = F::from_u32(padding_size as u32);

        if row_offset < num_rows {
            for padding_row in trace_rows.iter_mut().take(num_rows).skip(row_offset) {
                self.process_empty_slice(padding_row);
            }
            air_values.segment_last_seq_end = F::ONE;
            air_values.segment_last_src64 = F::ZERO;
            air_values.segment_last_dst64 = F::ZERO;
            air_values.segment_last_main_step = F::ZERO;
            air_values.segment_last_count64 = F::ZERO;
            air_values.last_count_chunk[0] = F::ZERO;
            air_values.last_count_chunk[1] = F::ZERO;
            air_values.segment_last_flags = F::ZERO;
        }

        // add range check of count to check that it's a positive 32-bits number
        let last_count = air_values.segment_last_count64.as_canonical_u64();
        local_16_bits_table[(last_count & 0xFFFF) as usize] += 1;
        local_16_bits_table[((last_count >> 16) & 0xFFFF) as usize] += 1;

        self.std.range_checks(self.range_16_bits_id, local_16_bits_table);

        let segment_id = segment_id.into();
        air_values.segment_id = F::from_usize(segment_id);
        air_values.is_last_segment = F::from_bool(is_last_segment);

        let first_input = flat_inputs.first().unwrap();
        if first_input.skip_rows == 0 {
            air_values.segment_previous_seq_end = F::ONE;
            air_values.segment_previous_dst64 = F::ZERO;
            air_values.segment_previous_src64 = F::ZERO;
            air_values.segment_previous_main_step = F::ZERO;
            air_values.segment_previous_count64 = F::ZERO;
            air_values.segment_previous_flags = F::ZERO;
        } else {
            assert!(segment_id > 0);
            air_values.segment_previous_seq_end = F::ZERO;
            air_values.segment_previous_dst64 =
                F::from_u32(trace_rows[0].get_dst64() - self.op_x_rows as u32);
            air_values.segment_previous_src64 =
                F::from_u32(trace_rows[0].get_src64() - self.op_x_rows as u32);
            air_values.segment_previous_main_step = F::from_u64(trace_rows[0].get_main_step());
            air_values.segment_previous_count64 =
                F::from_u32(trace_rows[0].get_count64() + self.op_x_rows as u32);
            air_values.segment_previous_flags = F::from_u16(F_SEL_MEMCPY as u16);
        }
        timer_stop_and_log_trace!(DMA_64_ALIGNED_TRACE);
        let from_trace = FromTrace::new(&mut trace).with_air_values(&mut air_values);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}
