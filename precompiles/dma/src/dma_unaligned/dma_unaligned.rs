use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

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

/// The state the last row of an input leaves behind: what the continuation of the segment hands
/// over when that row is also the last row of the instance.
///
/// Every input returns one, so the fill can run in parallel groups and the caller keeps the one of
/// the input that reaches the end of the trace.
struct SegmentEnd {
    seq_end: bool,
    src64: u32,
    dst64: u32,
    main_step: u64,
    count: u32,
    offset: u8,
    is_memeq: bool,
    /// The read the last write of the segment borrows its bytes from, 0 when the sequence ended.
    next_value: u64,
}

impl SegmentEnd {
    /// Writes the `segment_last_*` air values of a segment whose last row is this one.
    fn set_air_values<F: PrimeField64>(&self, air_values: &mut DmaUnalignedAirValues<F>) {
        if self.seq_end {
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
            air_values.segment_last_seq_end = F::ZERO;
            air_values.segment_last_src64 = F::from_u32(self.src64);
            air_values.segment_last_dst64 = F::from_u32(self.dst64);
            air_values.segment_last_main_step = F::from_u64(self.main_step);
            air_values.segment_last_count = F::from_u32(self.count);
            air_values.segment_last_offset = F::from_u8(self.offset);
            air_values.last_count_chunk[0] = F::from_u16(self.count as u16);
            air_values.last_count_chunk[1] = F::from_u16((self.count >> 16) as u16);
            air_values.segment_last_is_memeq = F::from_bool(self.is_memeq);
            for (index, byte) in air_values.segment_next_bytes.iter_mut().enumerate() {
                *byte = F::from_u8((self.next_value >> (index * 8)) as u8);
            }
        }
    }
}

/// What [`DmaUnalignedSM::fill_rows`] produces besides the rows themselves.
pub(crate) struct RowsFill {
    /// Multiplicities of the `DUAL_RANGE_BYTE` lookups of every lane, padding included.
    dual_byte_table: Vec<u64>,
    /// The state the last filled row leaves behind; what the continuation hands over when the
    /// instance has no padding.
    last: SegmentEnd,
    /// Rows left after the inputs, written as padding.
    padding_size: usize,
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

    /// Writes one input into `trace`, which is exactly `input.count` rows long, and charges the
    /// dual-byte range checks of its read bytes to `local_dual_byte_table`.
    ///
    /// The columns defined in the PIL with `<==` (`previous_seq_end`, `no_last_no_seq_end`,
    /// `write_value`) are *not* written here: they carry a `witness_calc` hint and the prover
    /// derives them from their expression before it commits. Every other column of the row is.
    #[inline(always)]
    fn process_input<R: DmaUnalignedTraceRowOps<F>>(
        input: &DmaUnalignedInput,
        trace: &mut [R],
        local_dual_byte_table: &mut [u64],
    ) -> SegmentEnd {
        // `input.count` and `input.skip` are in ROWS; the source values are indexed by slot.
        let rows = input.count as usize;
        debug_assert!(rows > 0 && rows == trace.len());
        let slots = input.get_input_slots();
        let skip_slots = input.skip as usize * DMA_UNALIGNED_OPS_BY_ROW;
        let initial_count = DmaInfo::get_loop_count(input.encoded) - skip_slots;
        let src_offset = DmaInfo::get_loop_src_offset(input.encoded);
        let mut dst64 = (input.dst >> 3) + skip_slots as u32;
        let mut src64 = (input.src >> 3) + skip_slots as u32;
        let addr_incr_by_row = DMA_UNALIGNED_OPS_BY_ROW as u32;

        let mut src_values_index = 0;
        let mut remaining = slots;
        let mut count = initial_count;
        let mut last_row_count = count;

        let mut seq_end = false;
        let mut next_value = 0;
        for row in trace.iter_mut() {
            row.set_main_step(input.step);
            row.set_is_memeq(input.is_mem_eq);

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
            last_row_count = count;

            // The sequence ends where @[count] runs out, NOT where this input's rows do: an input
            // that continues in the next instance fills its last row without ending anything.
            seq_end = count < DMA_UNALIGNED_OPS_BY_ROW;
            let used = if seq_end { count + 1 } else { DMA_UNALIGNED_OPS_BY_ROW };
            debug_assert!(used <= remaining, "row takes {used} slots of the {remaining} left");
            row.set_seq_end(seq_end);

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

        SegmentEnd {
            seq_end,
            src64: src64 - addr_incr_by_row,
            dst64: dst64 - addr_incr_by_row,
            main_step: input.step,
            count: last_row_count as u32,
            offset: src_offset,
            is_memeq: input.is_mem_eq,
            next_value,
        }
    }

    /// The row every unused row of the instance holds: all zero but @[seq_end], so it is not read
    /// as the continuation of a sequence. It uses lane 0 only -- @[sel_op] is a prefix and lane 0
    /// is always selected -- so it emits exactly one dummy load, as the unpacked air did with one
    /// per row.
    fn padding_row<R: DmaUnalignedTraceRowOps<F>>() -> R {
        let mut row = R::default();
        row.set_seq_end(true);
        row
    }

    /// Fills `rows` -- a whole instance -- with `flat_inputs` in order, then pads it to the end.
    ///
    /// The inputs are split into groups of about the same number of rows, and the trace is cut at
    /// the row each group starts on. An input is never split, so a group always owns whole inputs
    /// and its rows are contiguous; the groups keep the input order, so the last input of the last
    /// group is the one that ends the filled part of the trace.
    ///
    /// Takes `rows` rather than a `DmaUnalignedTrace` so the tests can run it over a handful of
    /// rows: the generated trace fixes 2^20 of them.
    pub(crate) fn fill_rows<R: DmaUnalignedTraceRowOps<F> + Copy + Send + Sync>(
        flat_inputs: &[&DmaUnalignedInput],
        rows: &mut [R],
    ) -> RowsFill {
        // `input.count` is already in rows: a sequence never shares a row with another, so the
        // planner rounds its slots up when it budgets them.
        let total_rows: usize = flat_inputs.iter().map(|input| input.count as usize).sum();
        let num_rows = rows.len();
        assert!(total_rows <= num_rows, "total_rows({total_rows}) > num_rows({num_rows})");
        assert!(total_rows > 0);

        let num_threads = rayon::current_num_threads().max(1);
        let rows_x_group = total_rows.div_ceil(num_threads).max(1);

        let (filled_rows, padding_rows) = rows.split_at_mut(total_rows);
        let mut groups: Vec<(&[&DmaUnalignedInput], &mut [R])> = Vec::new();
        let mut pending_inputs: &[&DmaUnalignedInput] = flat_inputs;
        let mut pending_rows: &mut [R] = filled_rows;
        while !pending_inputs.is_empty() {
            let mut take = 0;
            let mut group_rows = 0;
            while take < pending_inputs.len() && group_rows < rows_x_group {
                group_rows += pending_inputs[take].count as usize;
                take += 1;
            }
            let (group_inputs, rest_inputs) = pending_inputs.split_at(take);
            let (group_rows, rest_rows) = pending_rows.split_at_mut(group_rows);
            groups.push((group_inputs, group_rows));
            pending_inputs = rest_inputs;
            pending_rows = rest_rows;
        }

        // TODO: add std method to used short table, no sense with instances around 2^22 use 64 bits, need more space.
        let (tables, ends): (Vec<Vec<u64>>, Vec<SegmentEnd>) = groups
            .into_par_iter()
            .map(|(group_inputs, group_rows)| {
                let mut local_dual_byte_table = vec![0u64; 1 << 16];
                let mut cursor = 0usize;
                let mut end = None;
                for input in group_inputs {
                    let input_rows = input.count as usize;
                    end = Some(Self::process_input(
                        input,
                        &mut group_rows[cursor..cursor + input_rows],
                        &mut local_dual_byte_table,
                    ));
                    cursor += input_rows;
                }
                (local_dual_byte_table, end.expect("a group always holds at least one input"))
            })
            .unzip();

        let mut dual_byte_table = tables
            .into_iter()
            .reduce(|mut acc, table| {
                for (a, b) in acc.iter_mut().zip(table) {
                    *a += b;
                }
                acc
            })
            .expect("at least one group");
        let last = ends.into_iter().last().expect("at least one group");

        let padding_size = padding_rows.len();
        if padding_size > 0 {
            let padding_row = Self::padding_row::<R>();
            padding_rows.par_iter_mut().for_each(|row| *row = padding_row);
            // Every lane of a padding row reads zeros and is range-checked all the same.
            dual_byte_table[0] += (padding_size * DMA_UNALIGNED_OPS_BY_ROW * 4) as u64;
        }

        RowsFill { dual_byte_table, last, padding_size }
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `sctx` - The setup context containing the setup data.
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<R: DmaUnalignedTraceRowOps<F> + Copy + Send + Sync>(
        &self,
        inputs: &[Vec<DmaUnalignedInput>],
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        // Taken as-is, NOT zeroed: `process_input` writes every committed column of the rows it
        // fills, the padding rows are written whole by `fill_rows`, and the `<==` columns are
        // derived by the prover before it commits (see `mem_sm.rs` for the same reasoning), so
        // zeroing the whole trace first only moved hundreds of MB that were about to be written
        // again.
        let mut trace = DmaUnalignedTrace::<R>::new_from_vec(trace_buffer)?;
        let num_rows = trace.num_rows();

        let total_rows: usize = inputs
            .iter()
            .map(|inputs| inputs.iter().map(|input| input.count as usize).sum::<usize>())
            .sum();
        dma_trace("DmaUnaligned", total_rows, num_rows);

        timer_start_trace!(DMA_UNALIGNED_TRACE);

        let flat_inputs = crate::flatten_and_reorder_inputs(inputs);
        let fill = Self::fill_rows(&flat_inputs, trace.buffer.as_mut_slice());

        let mut air_values = DmaUnalignedAirValues::<F>::new();
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_last_segment = F::from_bool(is_last_segment);

        if fill.padding_size > 0 {
            air_values.padding_size = F::from_u32(fill.padding_size as u32);
            air_values.segment_last_seq_end = F::ONE;
            air_values.segment_last_src64 = F::ZERO;
            air_values.segment_last_dst64 = F::ZERO;
            air_values.segment_last_main_step = F::ZERO;
            air_values.segment_last_count = F::ZERO;
            air_values.segment_last_is_memeq = F::ZERO;
            air_values.segment_next_bytes = [F::ZERO; 8];
        } else {
            fill.last.set_air_values(&mut air_values);
        }

        // SECURITY: the count of the last row must be a positive 32-bit number, see the PIL.
        let last_count =
            if fill.padding_size == 0 && !fill.last.seq_end { fill.last.count } else { 0 };
        self.std.range_check_one(self.range_16_bits_id, last_count & 0xFFFF);
        self.std.range_check_one(self.range_16_bits_id, (last_count >> 16) & 0xFFFF);

        self.std.inc_virtual_rows_ranged(self.dual_range_byte_id, None, &fill.dual_byte_table);

        let trace_rows = trace.buffer.as_slice();
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

        timer_stop_and_log_trace!(DMA_UNALIGNED_TRACE);
        let from_trace = FromTrace::new(&mut trace).with_air_values(&mut air_values);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}

#[cfg(test)]
#[path = "../tests/dma_unaligned_witness_tests.rs"]
mod tests;
