use std::sync::Arc;

use proofman_fields::PrimeField64;

use crate::{dma_trace, DmaUnalignedInput, DMA_UNALIGNED_OPS_BY_ROW};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::SegmentId;
use zisk_pil::{
    DmaUnalignedAirValues, DmaUnalignedTrace, DmaUnalignedTraceRowOps, DUAL_RANGE_BYTE_ID,
};
use zisk_precomp_helpers::DmaInfo;

pub struct DmaUnalignedPrevSegment {
    pub seq_end: bool,
    pub dst64: u32,
    pub src64: u32,
    pub src_offset: u8,
    pub main_step: u64,
    pub count: u32,
    pub is_mem_eq: bool,
}

/// The `DmaUnalignedSM` struct encapsulates the logic of the DmaUnaligned State Machine.
pub struct DmaUnalignedSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Range checks ID's
    range_16_bits_id: usize,
    dual_range_byte_id: usize,
}

impl<F: PrimeField64> DmaUnalignedSM<F> {
    /// Creates a new Dma State Machine instance.
    ///
    /// # Returns
    /// A new `DmaUnalignedSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            dual_range_byte_id: std
                .get_virtual_table_id(DUAL_RANGE_BYTE_ID)
                .expect("Failed to get tabl eDUAL_RANGE_BYTE ID ID"),
            range_16_bits_id: std
                .get_range_id(0, 0xFFFF, None)
                .expect("Failed to get 16b table ID"),
        })
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_input<R: DmaUnalignedTraceRowOps<F>>(
        &self,
        input: &DmaUnalignedInput,
        trace: &mut [R],
        local_dual_byte_table: &mut [u64],
        air_values: &mut DmaUnalignedAirValues<F>,
    ) -> usize {
        // `input.count` and `input.skip` are in ROWS; the source values are indexed by slot.
        let rows = input.count as usize;
        let slots = input.get_input_slots();
        let skip_slots = input.skip as usize * DMA_UNALIGNED_OPS_BY_ROW;
        let is_last_instance_input = rows >= trace.len();
        let initial_count = DmaInfo::get_loop_count(input.encoded) - skip_slots;
        let src_offset = DmaInfo::get_loop_src_offset(input.encoded);
        let mut dst64 = (input.dst >> 3) + skip_slots as u32;
        let mut src64 = (input.src >> 3) + skip_slots as u32;
        let addr_incr_by_row = DMA_UNALIGNED_OPS_BY_ROW as u32;

        let mut src_values_index = 0;
        let mut remaining = slots;
        let mut count = initial_count;

        let mut seq_end = false;
        let mut next_value = 0;
        assert!(rows > 0);
        for row in trace.iter_mut().take(rows) {
            row.set_main_step(input.step);
            row.set_is_memeq(input.is_mem_eq);
            row.set_previous_seq_end(input.skip == 0 && src_values_index == 0);

            // Lane j works on src64 + j / dst64 + j, so the row only carries the base.
            row.set_dst64(dst64);
            row.set_src64(src64);
            dst64 += addr_incr_by_row;
            src64 += addr_incr_by_row;

            row.set_offset_2(src_offset == 2);
            row.set_offset_3(src_offset == 3);
            row.set_offset_4(src_offset == 4);
            row.set_offset_5(src_offset == 5);
            row.set_offset_6(src_offset == 6);
            row.set_offset_7(src_offset == 7);

            // @[count] is the count of lane 0 and falls by a whole row; the row that ends the
            // sequence is the one whose slots run out, and there `count == ops_selected - 1`.
            row.set_count(count as u32);

            // The sequence ends where @[count] runs out, NOT where this input's rows do: an input
            // that continues in the next instance fills its last row without ending anything.
            seq_end = count < DMA_UNALIGNED_OPS_BY_ROW;
            let used = if seq_end { count + 1 } else { DMA_UNALIGNED_OPS_BY_ROW };
            debug_assert!(used <= remaining, "row takes {used} slots of the {remaining} left");
            row.set_seq_end(seq_end);
            row.set_no_last_no_seq_end(!seq_end);

            let mut sel_op_from_1 = [false; DMA_UNALIGNED_OPS_BY_ROW - 1];
            let mut read_bytes = [[0u8; 8]; DMA_UNALIGNED_OPS_BY_ROW];

            for lane in 0..used {
                if lane > 0 {
                    sel_op_from_1[lane - 1] = true;
                }
                let value = input.src_values[src_values_index];
                src_values_index += 1;
                read_bytes[lane] = value.to_le_bytes();

                let value = value as usize;
                local_dual_byte_table[value & 0xFFFF] += 1;
                local_dual_byte_table[(value >> 16) & 0xFFFF] += 1;
                local_dual_byte_table[(value >> 32) & 0xFFFF] += 1;
                local_dual_byte_table[(value >> 48) & 0xFFFF] += 1;
            }

            // Every lane is range-checked whether it is selected or not, and an unselected one
            // reads zeros, so the lanes this row leaves over are counted here.
            local_dual_byte_table[0] += ((DMA_UNALIGNED_OPS_BY_ROW - used) * 4) as u64;

            row.set_all_sel_op_from_1(&sel_op_from_1);
            row.set_all_read_bytes(&read_bytes);

            // The bytes the last write of the segment borrows come from the slot after it, which
            // only exists while the sequence continues.
            next_value = if seq_end || src_values_index >= input.src_values.len() {
                0
            } else {
                input.src_values[src_values_index]
            };

            remaining -= used;
            count = count.saturating_sub(DMA_UNALIGNED_OPS_BY_ROW);
        }

        if is_last_instance_input {
            if seq_end {
                air_values.segment_last_seq_end = F::ONE;
                air_values.segment_last_src64 = F::ZERO;
                air_values.segment_last_dst64 = F::ZERO;
                air_values.segment_last_main_step = F::ZERO;
                air_values.segment_last_count = F::ZERO;
                air_values.segment_last_offset = F::ZERO;
                air_values.last_count_chunk[0] = F::ZERO;
                air_values.last_count_chunk[1] = F::ZERO;
                air_values.segment_last_is_memeq = F::ZERO;
                air_values.segment_next_bytes = [F::ZERO; 8];
            } else {
                let last_row = rows - 1;
                air_values.segment_last_seq_end = F::ZERO;
                air_values.segment_last_src64 = F::from_u32(trace[last_row].get_src64());
                air_values.segment_last_dst64 = F::from_u32(trace[last_row].get_dst64());
                air_values.segment_last_main_step = F::from_u64(trace[last_row].get_main_step());
                air_values.segment_last_count = F::from_u32(trace[last_row].get_count());
                air_values.segment_last_offset = F::from_u8(src_offset);
                let count = trace[last_row].get_count();
                air_values.last_count_chunk[0] = F::from_u16(count as u16);
                air_values.last_count_chunk[1] = F::from_u16((count >> 16) as u16);
                air_values.segment_last_is_memeq = F::from_bool(trace[last_row].get_is_memeq());
                for (index, byte) in air_values.segment_next_bytes.iter_mut().enumerate() {
                    *byte = F::from_u8((next_value >> (index * 8)) as u8);
                }
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
    pub fn process_empty_slice<R: DmaUnalignedTraceRowOps<F>>(&self, trace: &mut R) {
        trace.set_seq_end(true);
        trace.set_previous_seq_end(true);
        // A padding row uses lane 0 only — @[sel_op] is a prefix and lane 0 is always selected —
        // so it emits exactly one dummy load, as the unpacked air did with one per row.
        trace.set_all_sel_op_from_1(&[false; DMA_UNALIGNED_OPS_BY_ROW - 1]);
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `sctx` - The setup context containing the setup data.
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<R: DmaUnalignedTraceRowOps<F>>(
        &self,
        inputs: &[Vec<DmaUnalignedInput>],
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaUnalignedTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        // `input.count` is already in rows: a sequence never shares a row with another, so the
        // planner rounds its slots up when it budgets them.
        let total_rows: usize = inputs
            .iter()
            .map(|inputs| inputs.iter().map(|input| input.count as usize).sum::<usize>())
            .sum();

        assert!(total_rows <= num_rows, "total_rows({total_rows}) > num_rows({num_rows})");
        assert!(total_rows > 0);

        dma_trace("DmaUnaligned", total_rows, num_rows);

        timer_start_trace!(DMA_UNALIGNED_TRACE);

        let flat_inputs = crate::flatten_and_reorder_inputs(inputs);
        // Split the dma_trace.buffer into slices matching each inner vector’s length.
        let trace_rows = trace.buffer.as_mut_slice();

        // TODO: add std method to used short table, no sense with instances around 2^22 use 64 bits, need more space.
        let mut local_dual_byte_table = vec![0u64; 1 << 16];
        let mut air_values = DmaUnalignedAirValues::<F>::new();
        let mut row_offset = 0;
        for input in flat_inputs.iter() {
            let rows_used = self.process_input(
                input,
                &mut trace_rows[row_offset..],
                &mut local_dual_byte_table,
                &mut air_values,
            );
            row_offset += rows_used;
        }

        let padding_size = num_rows - row_offset;
        let last_count = if padding_size == 0 && !trace_rows[num_rows - 1].get_seq_end() {
            trace_rows[num_rows - 1].get_count()
        } else {
            0
        };
        self.std.range_check_one(self.range_16_bits_id, last_count & 0xFFFF);
        self.std.range_check_one(self.range_16_bits_id, (last_count >> 16) & 0xFFFF);

        // Every lane of a padding row reads zeros and is range-checked all the same.
        local_dual_byte_table[0] += (padding_size * DMA_UNALIGNED_OPS_BY_ROW * 4) as u64;
        self.std.inc_virtual_rows_ranged(self.dual_range_byte_id, None, &local_dual_byte_table);

        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_last_segment = F::from_bool(is_last_segment);

        let first_input = flat_inputs.first().unwrap();
        if first_input.skip == 0 {
            air_values.segment_previous_seq_end = F::ONE;
            air_values.segment_previous_dst64 = F::ZERO;
            air_values.segment_previous_src64 = F::ZERO;
            air_values.segment_previous_main_step = F::ZERO;
            air_values.segment_previous_count = F::ZERO;
            air_values.segment_previous_is_memeq = F::ZERO;
            air_values.segment_previous_offset = F::ZERO;
            air_values.segment_first_bytes = [F::ZERO; 8];
        } else {
            air_values.segment_previous_seq_end = F::ZERO;
            air_values.segment_previous_dst64 =
                F::from_u32(trace_rows[0].get_dst64() - DMA_UNALIGNED_OPS_BY_ROW as u32);
            air_values.segment_previous_src64 =
                F::from_u32(trace_rows[0].get_src64() - DMA_UNALIGNED_OPS_BY_ROW as u32);
            air_values.segment_previous_main_step = F::from_u64(trace_rows[0].get_main_step());
            // A row behind, not a slot: @[count] falls by a whole row.
            air_values.segment_previous_count =
                F::from_u32(trace_rows[0].get_count() + DMA_UNALIGNED_OPS_BY_ROW as u32);
            air_values.segment_previous_is_memeq = F::from_bool(trace_rows[0].get_is_memeq());
            air_values.segment_previous_offset =
                F::from_u8(DmaInfo::get_loop_src_offset(first_input.encoded));
            for (index, byte) in air_values.segment_first_bytes.iter_mut().enumerate() {
                // The bytes the previous segment lent us are the first read of the segment,
                // which is lane 0 of its first row.
                *byte = F::from_u8(trace_rows[0].get_read_bytes(0, index));
            }
        }

        // padding
        if padding_size > 0 {
            air_values.padding_size = F::from_u32(padding_size as u32);
            for row in trace_rows.iter_mut().take(num_rows).skip(row_offset) {
                self.process_empty_slice(row);
            }
            air_values.segment_last_seq_end = F::ONE;
            air_values.segment_last_src64 = F::ZERO;
            air_values.segment_last_dst64 = F::ZERO;
            air_values.segment_last_main_step = F::ZERO;
            air_values.segment_last_count = F::ZERO;
            air_values.segment_last_is_memeq = F::ZERO;
            air_values.segment_next_bytes = [F::ZERO; 8];
        }
        timer_stop_and_log_trace!(DMA_UNALIGNED_TRACE);
        let from_trace = FromTrace::new(&mut trace).with_air_values(&mut air_values);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}
