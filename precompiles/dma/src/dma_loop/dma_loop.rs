//! Witness of the `DmaLoop` air: the loop phase of any DMA operation, aligned or not.
//!
//! The same fill serves the loop block of `CompactDma` (see [`DmaLoopBlockRow`]), so what the air
//! values need is returned as [`DmaLoopSegmentValues`] and each air copies it into its own names.

use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::SegmentId;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{
    DmaLoopAirValues, DmaLoopTrace, DmaLoopTraceRow, DmaLoopTraceRowPacked, DUAL_RANGE_BYTE_ID,
};
use zisk_precomp_helpers::DmaInfo;

use crate::{
    dma_trace, set_dma_loop_offset, set_dma_loop_padding, DmaLoopBlockRow, DmaLoopInput,
    DMA_LOOP_OPS_BY_ROW,
};

/// Words of the sequence one row proves, `op_x_row` in the PIL.
const W: usize = DMA_LOOP_OPS_BY_ROW;

/// Bits of @[flags] (see `dma_loop.pil`).
const F_SEL_MEMCPY: u64 = 1;
const F_SEL_MEMCMP: u64 = 2;
const F_SEL_INPUTCPY: u64 = 4;
const F_SEL_MEMSET: u64 = 8;
const F_OFFSET: u64 = 16;

/// The row a sequence is at, as the continuation hands it over: the `segment_previous_*` /
/// `segment_last_*` payload of `dma_loop.pil`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DmaLoopRowState {
    pub seq_end: bool,
    pub src64: u32,
    pub dst64: u32,
    pub main_step: u64,
    pub count: u32,
    pub flags: u64,
    pub fill_byte: u8,
    /// Bytes of the first read of the next row: `segment_first_bytes` / `segment_next_bytes`.
    pub bytes: [u8; 8],
}

impl DmaLoopRowState {
    /// What a segment hands over or receives when no sequence crosses it: a finished sequence,
    /// with every other field forced to zero by the air.
    pub const ENDED: Self = Self {
        seq_end: true,
        src64: 0,
        dst64: 0,
        main_step: 0,
        count: 0,
        flags: 0,
        fill_byte: 0,
        bytes: [0; 8],
    };
}

/// Everything the air values of a loop segment are built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaLoopSegmentValues {
    pub segment_id: usize,
    pub is_last_segment: bool,
    pub previous: DmaLoopRowState,
    pub last: DmaLoopRowState,
    pub padding_size: usize,
}

impl DmaLoopSegmentValues {
    /// @[last_count_chunk]: the count of the last row, split to be range checked.
    pub fn last_count_chunks(&self) -> [u16; 2] {
        [self.last.count as u16, (self.last.count >> 16) as u16]
    }
}

/// What [`DmaLoopSM::fill_rows`] produces besides the rows.
pub struct DmaLoopFill {
    /// `DUAL_RANGE_BYTE` multiplicities of the active lanes (the air range checks no other).
    pub(crate) dual_byte_table: Vec<u64>,
    pub(crate) values: DmaLoopSegmentValues,
}

/// The `DmaLoopSM` struct encapsulates the logic of the DmaLoop State Machine.
pub struct DmaLoopSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    range_16_bits_id: usize,
    dual_range_byte_id: usize,
}

impl<F: PrimeField64> DmaLoopSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            dual_range_byte_id: std
                .get_virtual_table_id(DUAL_RANGE_BYTE_ID)
                .expect("Failed to get table DUAL_RANGE_BYTE ID"),
            range_16_bits_id: std
                .get_range_id(0, 0xFFFF, None)
                .expect("Failed to get 16b table ID"),
        })
    }

    /// @[flags] of the sequence an input belongs to.
    fn flags_of(input: &DmaLoopInput) -> u64 {
        let sel = match input.op {
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => F_SEL_MEMCPY,
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => F_SEL_MEMCMP,
            ZiskOp::DMA_INPUTCPY => F_SEL_INPUTCPY,
            ZiskOp::DMA_XMEMSET => F_SEL_MEMSET,
            op => panic!("Invalid DMA operation 0x{op:02X} in DmaLoop"),
        };
        sel + F_OFFSET * input.offset as u64
    }

    /// The state of the row `row` of the whole sequence of `input` (`skip` rows before this input
    /// included). `bytes` is left empty: it is the caller's business, it depends on which read the
    /// state is handed over with.
    fn row_state(input: &DmaLoopInput, row: usize) -> DmaLoopRowState {
        let class = input.class();
        let count = DmaInfo::get_loop_count(input.encoded) - row * W;
        let remaining = count + DmaLoopInput::is_unaligned_class(class) as usize;
        DmaLoopRowState {
            seq_end: remaining <= W,
            src64: input.src64 + (row * W) as u32,
            dst64: input.dst64 + (row * W) as u32,
            main_step: input.step,
            count: count as u32,
            flags: Self::flags_of(input),
            fill_byte: input.fill_byte,
            bytes: [0; 8],
        }
    }

    /// Writes one input into `rows`, which is exactly `input.rows` long, and charges the dual-byte
    /// range checks of its active lanes to `dual_byte_table`.
    ///
    /// The columns defined in the PIL with `<==` (`previous_seq_end`, `no_last_no_seq_end`,
    /// `write_value`, `tail_no_write`, `sel_read`, `sel_write`, `b0`, `extended_arg`) are *not*
    /// written: the prover derives them from their expression before it commits. Every other
    /// column is.
    fn process_input<R: DmaLoopBlockRow<F>>(
        input: &DmaLoopInput,
        rows: &mut [R],
        dual_byte_table: &mut [u64],
    ) {
        debug_assert_eq!(rows.len(), input.rows as usize);
        let is_memcpy = matches!(input.op, ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY);
        let is_memeq = matches!(input.op, ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP);
        let is_memset = input.op == ZiskOp::DMA_XMEMSET;
        let is_inputcpy = input.op == ZiskOp::DMA_INPUTCPY;
        let loads_count = input.loads_count() && input.skip == 0;

        let mut slot = 0usize;
        for (r, row) in rows.iter_mut().enumerate() {
            let state = Self::row_state(input, input.skip as usize + r);
            // The row that closes the sequence uses the slots it has left; every other one, all.
            let used = if state.seq_end {
                state.count as usize + DmaLoopInput::is_unaligned_class(input.class()) as usize
            } else {
                W
            };
            debug_assert!((1..=W).contains(&used));

            row.set_main_step(state.main_step);
            row.set_src64(state.src64);
            row.set_dst64(state.dst64);
            row.set_count(state.count);
            row.set_seq_end(state.seq_end);
            row.set_sel_memcpy(is_memcpy);
            row.set_sel_memeq(is_memeq);
            row.set_sel_memset(is_memset);
            row.set_sel_inputcpy(is_inputcpy);
            // Only the first row of a sequence may read the count, and only a direct memcpy does.
            row.set_sel_memcpy_count_load(loads_count && r == 0);
            row.set_fill_byte(state.fill_byte);
            set_dma_loop_offset::<F, R>(row, input.offset);

            let sel_op_from_1: [bool; W - 1] = std::array::from_fn(|lane| lane + 1 < used);
            row.set_all_sel_op_from_1(&sel_op_from_1);

            // A memset "reads" the fill byte on every lane, the inactive ones included -- the air
            // asks it of the whole row. Every other operation leaves an inactive lane at zero.
            let mut read_bytes = [if is_memset { input.fill_byte } else { 0 }; 8 * W];
            for lane in 0..used {
                let value = input.value(slot);
                slot += 1;
                read_bytes[8 * lane..8 * lane + 8].copy_from_slice(&value.to_le_bytes());
                // The air range checks the active lanes only (`sel_op[i]` is the selector).
                dual_byte_table[(value & 0xFFFF) as usize] += 1;
                dual_byte_table[((value >> 16) & 0xFFFF) as usize] += 1;
                dual_byte_table[((value >> 32) & 0xFFFF) as usize] += 1;
                dual_byte_table[((value >> 48) & 0xFFFF) as usize] += 1;
            }
            row.set_all_read_bytes(&read_bytes);
        }
        debug_assert_eq!(slot, input.slots(), "the rows took a different number of slots");
    }

    /// Fills `rows` -- a whole instance, or the loop block of one -- with `flat_inputs` in order,
    /// then pads it to the end, and returns what the air values and the range checks need.
    ///
    /// The inputs are split into groups of about the same number of rows and the trace is cut at
    /// the row each group starts on. An input is never split, so a group always owns whole inputs
    /// and its rows are contiguous. The inputs have to come in the order
    /// [`crate::flatten_and_reorder_inputs`] gives them: the one that continues the previous
    /// instance first, the one that goes on in the next instance last.
    ///
    /// Takes `rows` rather than a `DmaLoopTrace` because the fused `CompactDma` air carries this
    /// block inside a wider row, and so the tests can run it over a handful of rows.
    pub(crate) fn fill_rows<R: DmaLoopBlockRow<F>>(
        flat_inputs: &[&DmaLoopInput],
        rows: &mut [R],
        segment_id: SegmentId,
        is_last_segment: bool,
    ) -> DmaLoopFill {
        let total_rows: usize = flat_inputs.iter().map(|input| input.rows as usize).sum();
        let num_rows = rows.len();
        assert!(total_rows <= num_rows, "DmaLoop: {total_rows} rows do not fit in {num_rows}");
        debug_assert!(
            flat_inputs.iter().skip(1).all(|input| input.skip == 0),
            "only the first input may continue a previous instance"
        );
        debug_assert!(
            flat_inputs.iter().rev().skip(1).all(|input| !input.is_last_instance_input),
            "only the last input may go on in the next instance"
        );

        let num_threads = rayon::current_num_threads().max(1);
        let rows_x_group = total_rows.div_ceil(num_threads).max(1);

        let (filled_rows, padding_rows) = rows.split_at_mut(total_rows);
        let mut groups: Vec<(&[&DmaLoopInput], &mut [R])> = Vec::new();
        let mut pending_inputs: &[&DmaLoopInput] = flat_inputs;
        let mut pending_rows: &mut [R] = filled_rows;
        while !pending_inputs.is_empty() {
            let mut take = 0;
            let mut group_rows = 0;
            while take < pending_inputs.len() && group_rows < rows_x_group {
                group_rows += pending_inputs[take].rows as usize;
                take += 1;
            }
            let (group_inputs, rest_inputs) = pending_inputs.split_at(take);
            let (group_rows, rest_rows) = pending_rows.split_at_mut(group_rows);
            groups.push((group_inputs, group_rows));
            pending_inputs = rest_inputs;
            pending_rows = rest_rows;
        }

        let tables: Vec<Vec<u64>> = groups
            .into_par_iter()
            .map(|(group_inputs, group_rows)| {
                let mut table = vec![0u64; 1 << 16];
                let mut cursor = 0usize;
                for input in group_inputs {
                    let input_rows = input.rows as usize;
                    Self::process_input(
                        input,
                        &mut group_rows[cursor..cursor + input_rows],
                        &mut table,
                    );
                    cursor += input_rows;
                }
                table
            })
            .collect();
        let dual_byte_table = tables
            .into_iter()
            .reduce(|mut acc, table| {
                for (a, b) in acc.iter_mut().zip(table) {
                    *a += b;
                }
                acc
            })
            .unwrap_or_else(|| vec![0u64; 1 << 16]);

        // Padding: one row, copied into every row the inputs left -- its loop block only, the
        // rest of a `CompactDma` row belongs to the other fill. It has no active lane, so it
        // raises no range check.
        let padding_size = padding_rows.len();
        if padding_size > 0 {
            let mut padding = R::default();
            set_dma_loop_padding::<F, R>(&mut padding);
            padding_rows.par_iter_mut().for_each(|row| row.copy_block_from(&padding));
        }

        let values = Self::segment_values(flat_inputs, padding_size, segment_id, is_last_segment);
        DmaLoopFill { dual_byte_table, values }
    }

    /// The continuation values of a segment, from the inputs it holds.
    fn segment_values(
        flat_inputs: &[&DmaLoopInput],
        padding_size: usize,
        segment_id: SegmentId,
        is_last_segment: bool,
    ) -> DmaLoopSegmentValues {
        // What the previous segment handed over: nothing, unless the first input continues a
        // sequence it cut. Then it is the row before this segment's first one, with the bytes of
        // the read this segment starts with.
        let previous = match flat_inputs.first() {
            Some(first) if first.skip > 0 => {
                let mut state = Self::row_state(first, first.skip as usize - 1);
                state.bytes = first.value(0).to_le_bytes();
                state
            }
            _ => DmaLoopRowState::ENDED,
        };

        // What this segment hands over: nothing when it ends on padding or on the last row of a
        // sequence; the last row otherwise, with the bytes of the read the next segment starts
        // with, which its last write borrows from.
        let last = match flat_inputs.last() {
            Some(last) if padding_size == 0 => {
                let mut state = Self::row_state(last, last.skip as usize + last.rows as usize - 1);
                if state.seq_end {
                    DmaLoopRowState::ENDED
                } else {
                    debug_assert!(last.is_last_instance_input);
                    state.bytes = last.value(last.slots()).to_le_bytes();
                    state
                }
            }
            _ => DmaLoopRowState::ENDED,
        };

        DmaLoopSegmentValues {
            segment_id: segment_id.into(),
            is_last_segment,
            previous,
            last,
            padding_size,
        }
    }

    /// Raises in the `Std` the multiplicities a fill returned: the dual-byte lookups of its lanes
    /// and the range check of @[last_count_chunk].
    pub(crate) fn charge(&self, fill: &DmaLoopFill) {
        self.std.inc_virtual_rows_ranged(self.dual_range_byte_id, None, &fill.dual_byte_table);
        for chunk in fill.values.last_count_chunks() {
            self.std.range_check_one(self.range_16_bits_id, chunk as u64);
        }
    }

    fn compute_witness_inner<R: DmaLoopBlockRow<F>>(
        &self,
        inputs: &[Vec<DmaLoopInput>],
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        // Taken as-is, NOT zeroed: `fill_rows` writes every committed column of every row, and the
        // `<==` ones are derived by the prover before it commits.
        let mut trace = DmaLoopTrace::<R>::new_from_vec(trace_buffer)?;
        let num_rows = trace.num_rows();

        let flat_inputs = crate::flatten_and_reorder_inputs(inputs);
        let total_rows: usize = flat_inputs.iter().map(|input| input.rows as usize).sum();
        dma_trace("DmaLoop", total_rows, num_rows);

        timer_start_trace!(DMA_LOOP_TRACE);
        let fill =
            Self::fill_rows(&flat_inputs, trace.buffer.as_mut_slice(), segment_id, is_last_segment);
        self.charge(&fill);
        timer_stop_and_log_trace!(DMA_LOOP_TRACE);

        let mut air_values = DmaLoopAirValues::<F>::new();
        let v = &fill.values;
        air_values.segment_id = F::from_usize(v.segment_id);
        air_values.is_last_segment = F::from_bool(v.is_last_segment);
        air_values.padding_size = F::from_usize(v.padding_size);
        air_values.segment_previous_seq_end = F::from_bool(v.previous.seq_end);
        air_values.segment_previous_src64 = F::from_u32(v.previous.src64);
        air_values.segment_previous_dst64 = F::from_u32(v.previous.dst64);
        air_values.segment_previous_main_step = F::from_u64(v.previous.main_step);
        air_values.segment_previous_count = F::from_u32(v.previous.count);
        air_values.segment_previous_flags = F::from_u64(v.previous.flags);
        air_values.segment_previous_fill_byte = F::from_u8(v.previous.fill_byte);
        air_values.segment_first_bytes = v.previous.bytes.map(F::from_u8);
        air_values.segment_last_seq_end = F::from_bool(v.last.seq_end);
        air_values.segment_last_src64 = F::from_u32(v.last.src64);
        air_values.segment_last_dst64 = F::from_u32(v.last.dst64);
        air_values.segment_last_main_step = F::from_u64(v.last.main_step);
        air_values.segment_last_count = F::from_u32(v.last.count);
        air_values.segment_last_flags = F::from_u64(v.last.flags);
        air_values.segment_last_fill_byte = F::from_u8(v.last.fill_byte);
        air_values.segment_next_bytes = v.last.bytes.map(F::from_u8);
        air_values.last_count_chunk = v.last_count_chunks().map(F::from_u16);

        let from_trace = FromTrace::new(&mut trace).with_air_values(&mut air_values);
        Ok(AirInstance::new_from_trace(from_trace))
    }

    /// Computes the witness of one `DmaLoop` instance.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<DmaLoopInput>],
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<DmaLoopTraceRowPacked<F>>(
                inputs,
                segment_id,
                is_last_segment,
                trace_buffer,
            )
        } else {
            self.compute_witness_inner::<DmaLoopTraceRow<F>>(
                inputs,
                segment_id,
                is_last_segment,
                trace_buffer,
            )
        }
    }
}

#[cfg(test)]
#[path = "../tests/dma_loop_witness_tests.rs"]
mod tests;
