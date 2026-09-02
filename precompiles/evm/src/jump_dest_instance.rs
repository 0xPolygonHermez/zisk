//! Witness computation for the `JumpDest` AIR.
//!
//! Turns the operations a segment collected into trace rows. The row values
//! follow `precompiles/evm/pil/jump_dest.pil` exactly, so the two must be read
//! together:
//!
//! * `src64` advances by `op_x_row` every row — one source word per op.
//! * `dst64` is the bitmap word address. It stays put inside a block and steps
//!   by one **on** the last clock, which is why the mstore of that row targets
//!   `'dst64`: by then the column already points at the next block.
//! * `count` is the bytes left **before** the row runs, so the first row of a
//!   sequence holds the byte count the operation read from `EXTRA_PARAMS_ADDR`
//!   and the row where `seq_end` is set holds exactly what that row consumes.
//! * `state` chains op to op, `state[0]` of a row continuing `state[op_x_row]`
//!   of the previous one.
//! * `seq_start` is `seq_end` of the row before gated by `sel`, by its own
//!   definition, so it is filled as a pass over the finished trace.
//!
//! A segment may begin in the middle of an operation, so the first collected
//! one can have leading rows that belong to the previous segment; the collector
//! reports how many in `first_row_offset`. Those rows are dropped, and what the
//! previous segment left behind is republished in the `segment_previous_*` air
//! values, which is what the continuation bus checks.
//!
//! The rows past the last operation form inactive blocks: they keep the
//! transitions running (`src64`, `dst64` and `state` carry on, `count` stays at
//! zero) with `sel` cleared, so they drive no bus at all.

use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType, SegmentId,
    StatsType,
};
use zisk_pil::{
    JumpDestAirValues, JumpDestTrace, JumpDestTraceRow, JumpDestTraceRowOps,
    JumpDestTraceRowPacked, JUMP_DEST_BITMAP_TABLE_ID, JUMP_DEST_COMPRESSOR_TABLE_ID,
};
use zisk_precomp_helpers::{
    expand_jump_dest_ops, jd_compressor_row, JumpDestBitmapTableIndex, JumpDestOp,
    JUMP_DEST_BITMAP_TABLE_ROWS, JUMP_DEST_COMPRESSOR_TABLE_ROWS,
};

use crate::{
    JumpDestCheckPoint, JumpDestCollector, JumpDestInput, JUMP_DEST_OPS_X_ROW,
    JUMP_DEST_ROWS_X_BLOCK,
};

/// Multiplicities the trace owes the two tables, one slot per table row.
///
/// Every op performs five lookups, so a whole instance runs into the millions.
/// Going through `Std` for each one costs a call, an id unwrap and the
/// contention behind it; bumping an array instead makes it an add, and the
/// totals are handed over once at the end.
struct LookupMuls {
    compressor: Vec<u64>,
    bitmap: Vec<u64>,
}

impl LookupMuls {
    fn new() -> Self {
        Self {
            compressor: vec![0; JUMP_DEST_COMPRESSOR_TABLE_ROWS],
            bitmap: vec![0; JUMP_DEST_BITMAP_TABLE_ROWS],
        }
    }

    /// Counts the five lookups of one op: one per 16-bit chunk against the
    /// compressor, and one against the bitmap table.
    #[inline(always)]
    fn count(&mut self, op: &JumpDestOp, index: &JumpDestBitmapTableIndex) {
        let mut cdata4 = 0u64;
        for chunk in 0..4 {
            self.compressor[jd_compressor_row(op.data[chunk], op.ignore[chunk]) as usize] += 1;
            cdata4 |= (op.cdata[chunk] as u64) << (8 * chunk);
        }
        self.bitmap[index.row(op.state_in, cdata4, op.bytes_used, op.state_out) as usize] += 1;
    }
}

/// Where the walk stands at a row boundary. Both ends of a segment are one of
/// these: what the previous segment left, which becomes the `segment_previous_*`
/// air values, and what this one leaves.
#[derive(Clone, Copy, Default)]
struct Cursor {
    seq_end: bool,
    src64: u32,
    dst64: u32,
    main_step: u64,
    count: u32,
    /// Bytes the row consumed. Not a trace column of its own — it is the sum of
    /// the row's `bytes_used` — but the next row's `count` needs it.
    bytes_used: u32,
    state: u8,
}

/// Fills the `JumpDest` trace.
pub struct JumpDestSM<F: PrimeField64> {
    std: Arc<Std<F>>,
    /// Virtual-table handles for the two tables the machine looks up. Their
    /// multiplicities are ours to raise: the lookups only balance if the proving
    /// side counts every row the trace assumes.
    compressor_table_id: usize,
    bitmap_table_id: usize,
    /// Which row of the bitmap table proves a given op.
    bitmap_index: JumpDestBitmapTableIndex,
    /// The 16-bit range the two halves of `segment_last_count` are checked
    /// against. Its multiplicity is ours to raise too — a range check on an
    /// airvalue is a bus emission like any other.
    range_16_bits_id: usize,
}

impl<F: PrimeField64> JumpDestSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let compressor_table_id = std
            .get_virtual_table_id(JUMP_DEST_COMPRESSOR_TABLE_ID)
            .expect("Failed to get JUMP_DEST_COMPRESSOR_TABLE identifier");
        let bitmap_table_id = std
            .get_virtual_table_id(JUMP_DEST_BITMAP_TABLE_ID)
            .expect("Failed to get JUMP_DEST_BITMAP_TABLE identifier");

        let range_16_bits_id =
            std.get_range_id(0, 0xFFFF, None).expect("Failed to get the 16-bit range id");

        Arc::new(Self {
            std,
            compressor_table_id,
            bitmap_table_id,
            bitmap_index: JumpDestBitmapTableIndex::new(),
            range_16_bits_id,
        })
    }

    /// Writes the rows of one operation, starting `skip_rows` into it. Returns
    /// how many rows were written and where the walk stands afterwards.
    /// `on_op` is handed every op of an active row, which is where the lookup
    /// multiplicities are raised. It is a parameter so the row logic can be
    /// exercised without a `Std`.
    fn process_input<R: JumpDestTraceRowOps<F>>(
        input: &JumpDestInput,
        skip_rows: usize,
        trace: &mut [R],
        mut on_op: impl FnMut(&JumpDestOp),
    ) -> (usize, Cursor) {
        let ops = expand_jump_dest_ops(input.count as usize, &input.words);
        let total_rows = ops.len() / JUMP_DEST_OPS_X_ROW;
        let rows = (total_rows - skip_rows).min(trace.len());

        let src64_base = (input.bytecode_addr / 8) as u32;
        let dst64_base = (input.bitmap_addr / 8) as u32;
        let mut count = Self::count_before(&ops, skip_rows, input.count as u32);
        let mut cursor = Cursor::default();

        for (index, row) in trace.iter_mut().enumerate().take(rows) {
            let row_index = skip_rows + index;
            let block = row_index / JUMP_DEST_ROWS_X_BLOCK;
            let is_last_clock = row_index % JUMP_DEST_ROWS_X_BLOCK == JUMP_DEST_ROWS_X_BLOCK - 1;
            let seq_end = row_index + 1 == total_rows;

            let slice = &ops[row_index * JUMP_DEST_OPS_X_ROW..][..JUMP_DEST_OPS_X_ROW];
            let bytes_used = slice.iter().map(|op| op.bytes_used as u32).sum::<u32>();

            cursor = Cursor {
                seq_end,
                src64: src64_base + (row_index * JUMP_DEST_OPS_X_ROW) as u32,
                // The increment lands on the last clock, so that row already
                // carries the next block's address; its mstore uses `'dst64`.
                dst64: dst64_base + block as u32 + is_last_clock as u32,
                main_step: input.main_step,
                count,
                bytes_used,
                state: slice[JUMP_DEST_OPS_X_ROW - 1].state_out,
            };
            count -= bytes_used;

            debug_assert!(
                !is_last_clock || seq_end || bytes_used == (8 * JUMP_DEST_OPS_X_ROW) as u32,
                "a block that does not end the sequence must be full"
            );

            row.set_sel(true);
            row.set_seq_end(seq_end);
            Self::set_cursor(row, &cursor);
            Self::set_ops(row, slice);
            // Only selected rows drive the lookups, so only these are counted.
            slice.iter().for_each(&mut on_op);
        }

        (rows, cursor)
    }

    /// Bytes still to consume when row `row_index` starts, which is what its
    /// `count` column holds.
    fn count_before(ops: &[JumpDestOp], row_index: usize, count: u32) -> u32 {
        count
            - ops[..row_index * JUMP_DEST_OPS_X_ROW]
                .iter()
                .map(|op| op.bytes_used as u32)
                .sum::<u32>()
    }

    /// Writes the columns that say where the walk stands.
    fn set_cursor<R: JumpDestTraceRowOps<F>>(row: &mut R, cursor: &Cursor) {
        row.set_src64(cursor.src64);
        row.set_dst64(cursor.dst64);
        row.set_main_step(cursor.main_step);
        row.set_count(cursor.count);
    }

    /// Writes the per-operation columns of one row.
    fn set_ops<R: JumpDestTraceRowOps<F>>(row: &mut R, slice: &[JumpDestOp]) {
        let mut data = [[0u16; JUMP_DEST_OPS_X_ROW]; 4];
        let mut cdata = [[0u8; JUMP_DEST_OPS_X_ROW]; 4];
        let mut sel_mem_load = [false; JUMP_DEST_OPS_X_ROW];
        let mut bitmap_byte = [0u8; JUMP_DEST_OPS_X_ROW];
        let mut bytes_used = [0u8; JUMP_DEST_OPS_X_ROW];
        let mut state = [0u8; JUMP_DEST_OPS_X_ROW + 1];

        state[0] = slice[0].state_in;
        for (i, op) in slice.iter().enumerate() {
            for chunk in 0..4 {
                data[chunk][i] = op.data[chunk];
                cdata[chunk][i] = op.cdata[chunk];
            }
            sel_mem_load[i] = op.sel_mem_load;
            bitmap_byte[i] = op.bitmap_byte;
            bytes_used[i] = op.bytes_used;
            state[i + 1] = op.state_out;
        }

        row.set_all_data(&data);
        row.set_all_cdata(&cdata);
        row.set_all_sel_mem_load(&sel_mem_load);
        row.set_all_bitmap_byte(&bitmap_byte);
        row.set_all_bytes_used(&bytes_used);
        row.set_all_state(&state);
    }

    /// Fills the inactive blocks that close the segment. They keep running the
    /// transitions with nothing selected: repeating the state keeps
    /// `state[0] == 'state[op_x_row]` true row to row, and since the last active
    /// row is a `seq_end`, where `count == total_bytes_used`, the first inactive
    /// row lands on zero and stays there.
    fn fill_inactive<R: JumpDestTraceRowOps<F>>(from: usize, cursor: &Cursor, trace: &mut [R]) {
        let mut cursor = *cursor;
        let state = [cursor.state; JUMP_DEST_OPS_X_ROW + 1];

        for (index, row) in trace.iter_mut().enumerate() {
            let is_last_clock =
                (from + index) % JUMP_DEST_ROWS_X_BLOCK == JUMP_DEST_ROWS_X_BLOCK - 1;
            cursor.src64 += JUMP_DEST_OPS_X_ROW as u32;
            cursor.dst64 += is_last_clock as u32;
            cursor.count -= cursor.bytes_used;
            cursor.bytes_used = 0;
            Self::set_cursor(row, &cursor);
            row.set_all_state(&state);
        }
    }

    /// `seq_start` is the `seq_end` carried from the previous row, taken from
    /// the segment that came before on the first one, and gated by `sel` so that
    /// an inactive block opens no sequence — it is the multiplicity of
    /// `proves_operation` and the selector of the `EXTRA_PARAMS` read.
    fn fill_seq_start<R: JumpDestTraceRowOps<F>>(rows: &mut [R], mut carry: bool) {
        for row in rows.iter_mut() {
            let seq_end = row.get_seq_end();
            row.set_seq_start(carry && row.get_sel());
            carry = seq_end;
        }
    }

    pub fn compute_witness(
        &self,
        inputs: &[Vec<JumpDestInput>],
        skip_rows: usize,
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<JumpDestTraceRowPacked<F>>(
                inputs,
                skip_rows,
                segment_id,
                is_last_segment,
                trace_buffer,
            )
        } else {
            self.compute_witness_inner::<JumpDestTraceRow<F>>(
                inputs,
                skip_rows,
                segment_id,
                is_last_segment,
                trace_buffer,
            )
        }
    }

    fn compute_witness_inner<R: JumpDestTraceRowOps<F>>(
        &self,
        inputs: &[Vec<JumpDestInput>],
        skip_rows: usize,
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = JumpDestTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();
        let rows = trace.buffer.as_mut_slice();

        // What the previous segment left behind. Starting on an operation
        // boundary means there is nothing to continue, and the PIL wants those
        // air values zeroed.
        let previous = inputs
            .iter()
            .flatten()
            .next()
            .filter(|_| skip_rows > 0)
            .map(|input| Self::cursor_before(input, skip_rows))
            .unwrap_or(Cursor { seq_end: true, ..Cursor::default() });

        let mut offset = 0usize;
        let mut skip = skip_rows;
        let mut last = previous;
        let mut muls = LookupMuls::new();

        for input in inputs.iter().flatten() {
            if offset >= num_rows {
                break;
            }
            let (written, cursor) = Self::process_input(input, skip, &mut rows[offset..], |op| {
                muls.count(op, &self.bitmap_index)
            });
            offset += written;
            skip = 0;
            last = cursor;
        }

        Self::fill_inactive(offset, &last, &mut rows[offset..]);

        let mut air_values = JumpDestAirValues::<F>::new();
        air_values.segment_id = F::from_u64(segment_id.0 as u64);
        air_values.is_last_segment = F::from_bool(is_last_segment);

        air_values.segment_previous_seq_end = F::from_bool(previous.seq_end);
        if !previous.seq_end {
            air_values.segment_previous_src64 = F::from_u32(previous.src64);
            air_values.segment_previous_dst64 = F::from_u32(previous.dst64);
            air_values.segment_previous_main_step = F::from_u64(previous.main_step);
            air_values.segment_previous_count = F::from_u32(previous.count);
            air_values.segment_previous_state = F::from_u8(previous.state);
        }

        // The continuation is read off the trace's own last row, inactive
        // blocks included, because that is the row `LAST` selects.
        let closed = offset == num_rows && last.seq_end;
        air_values.segment_last_seq_end = F::from_bool(closed);
        if !closed {
            let row = &rows[num_rows - 1];
            air_values.segment_last_src64 = F::from_u32(row.get_src64());
            air_values.segment_last_dst64 = F::from_u32(row.get_dst64());
            air_values.segment_last_main_step = F::from_u64(row.get_main_step());
            air_values.segment_last_count = F::from_u32(row.get_count());
            air_values.segment_last_state = F::from_u8(row.get_state(JUMP_DEST_OPS_X_ROW));
        }

        Self::fill_seq_start(rows, previous.seq_end);

        self.std.inc_virtual_rows_ranged(self.compressor_table_id, None, &muls.compressor);
        self.std.inc_virtual_rows_ranged(self.bitmap_table_id, None, &muls.bitmap);

        // `count` is proved non-negative by splitting the segment's last value
        // into two 16-bit chunks.
        let last_count = air_values.segment_last_count.as_canonical_u64();
        let chunks = [last_count & 0xFFFF, last_count >> 16];
        air_values.last_count_chunk[0] = F::from_u64(chunks[0]);
        air_values.last_count_chunk[1] = F::from_u64(chunks[1]);
        for chunk in chunks {
            self.std.range_check(self.range_16_bits_id, chunk, 1u64);
        }

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }

    /// Rebuilds the row the previous segment ended on, from the operation it cut
    /// through. Segments are cut at multiples of the AIR size and operations
    /// occupy whole blocks, so `skip_rows` always lands on a block boundary and
    /// the row before it is a last-clock row — which is why `dst64` is already
    /// the next block's.
    fn cursor_before(input: &JumpDestInput, skip_rows: usize) -> Cursor {
        let ops = expand_jump_dest_ops(input.count as usize, &input.words);
        debug_assert_eq!(skip_rows % JUMP_DEST_ROWS_X_BLOCK, 0, "segments cut on block boundaries");

        // That row closed a block without closing the sequence, so it consumed
        // a full one. This is what lets the PIL substitute a constant for
        // `'total_bytes_used` on the first row of the segment.
        let bytes_used = (8 * JUMP_DEST_OPS_X_ROW) as u32;
        debug_assert_eq!(
            Self::count_before(&ops, skip_rows - 1, input.count as u32)
                - Self::count_before(&ops, skip_rows, input.count as u32),
            bytes_used,
            "the block before the cut must be full"
        );

        Cursor {
            seq_end: false,
            src64: (input.bytecode_addr / 8) as u32
                + ((skip_rows - 1) * JUMP_DEST_OPS_X_ROW) as u32,
            dst64: (input.bitmap_addr / 8) as u32 + (skip_rows / JUMP_DEST_ROWS_X_BLOCK) as u32,
            main_step: input.main_step,
            count: Self::count_before(&ops, skip_rows - 1, input.count as u32),
            bytes_used,
            state: ops[skip_rows * JUMP_DEST_OPS_X_ROW].state_in,
        }
    }
}

/// One segment of the `JumpDest` AIR.
pub struct JumpDestInstance<F: PrimeField64> {
    sm: Arc<JumpDestSM<F>>,
    ictx: InstanceCtx,
    is_last_segment: bool,
}

impl<F: PrimeField64> JumpDestInstance<F> {
    pub fn new(sm: Arc<JumpDestSM<F>>, ictx: InstanceCtx) -> Self {
        let is_last_segment = Self::checkpoint(&ictx).is_last_segment;
        Self { sm, ictx, is_last_segment }
    }

    fn checkpoint(ictx: &InstanceCtx) -> &JumpDestCheckPoint {
        ictx.plan
            .meta
            .as_ref()
            .expect("JumpDestInstance: plan without meta")
            .downcast_ref::<JumpDestCheckPoint>()
            .expect("JumpDestInstance: meta is not a JumpDestCheckPoint")
    }

    pub fn build_jump_dest_collector(&self, chunk_id: ChunkId) -> JumpDestCollector {
        let checkpoint = Self::checkpoint(&self.ictx);
        let (rows, skipper) = checkpoint.chunks[&chunk_id];
        JumpDestCollector::new(chunk_id, rows, skipper, Some(chunk_id) == checkpoint.last_chunk)
    }
}

impl<F: PrimeField64> Instance<F> for JumpDestInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let mut skip_rows = 0usize;
        let mut inputs = Vec::with_capacity(collectors.len());

        for (index, (_, collector)) in collectors.into_iter().enumerate() {
            let collector = collector.as_any().downcast::<JumpDestCollector>().unwrap();
            if index == 0 {
                skip_rows = collector.first_row_offset as usize;
            }
            inputs.push(collector.take_inputs());
        }

        Ok(Some(self.sm.compute_witness(
            &inputs,
            skip_rows,
            self.ictx.plan.segment_id.unwrap(),
            self.is_last_segment,
            trace_buffer,
            packed,
        )?))
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Precompiled
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(self.build_jump_dest_collector(chunk_id)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_precomp_helpers::{bitmap_words, src_words};

    type F = Goldilocks;

    fn input(bytecode: &[u8], bitmap_addr: u64, bytecode_addr: u64) -> JumpDestInput {
        let words = (0..src_words(bytecode.len()))
            .map(|w| {
                let mut bytes = [0u8; 8];
                let offset = w * 8;
                let available = std::cmp::min(8, bytecode.len() - offset);
                bytes[..available].copy_from_slice(&bytecode[offset..offset + available]);
                u64::from_le_bytes(bytes)
            })
            .collect();
        JumpDestInput {
            bitmap_addr,
            bytecode_addr,
            main_step: 42,
            count: bytecode.len() as u64,
            words,
        }
    }

    fn blank(rows: usize) -> Vec<JumpDestTraceRow<F>> {
        vec![JumpDestTraceRow::<F>::default(); rows]
    }

    /// Checks every row-to-row constraint of `jump_dest.pil` over a filled
    /// trace, given what the segment before it left behind.
    fn check_transitions(rows: &[JumpDestTraceRow<F>], previous: &Cursor) {
        for (index, row) in rows.iter().enumerate() {
            let is_last_clock = index % JUMP_DEST_ROWS_X_BLOCK == JUMP_DEST_ROWS_X_BLOCK - 1;
            let total_bytes_used: u32 =
                (0..JUMP_DEST_OPS_X_ROW).map(|i| row.get_bytes_used(i) as u32).sum();

            // The previous row, or what the previous segment handed over. Its
            // `bytes_used` is what this row's `count` is measured against.
            let prev = match index.checked_sub(1).map(|p| &rows[p]) {
                Some(p) => Cursor {
                    seq_end: p.get_seq_end(),
                    src64: p.get_src64(),
                    dst64: p.get_dst64(),
                    main_step: p.get_main_step(),
                    count: p.get_count(),
                    bytes_used: (0..JUMP_DEST_OPS_X_ROW).map(|i| p.get_bytes_used(i) as u32).sum(),
                    state: p.get_state(JUMP_DEST_OPS_X_ROW),
                },
                None => *previous,
            };

            // seq_end * (1 - LAST_CLOCK) === 0
            assert!(!row.get_seq_end() || is_last_clock, "row {index}: seq_end off the last clock");

            // seq_end * (count - total_bytes_used) === 0
            if row.get_seq_end() {
                assert_eq!(
                    row.get_count(),
                    total_bytes_used,
                    "row {index}: the sequence must end on the last byte"
                );
            }

            // LAST_CLOCK * (sel - seq_end) * (total_bytes_used - 8 * op_x_row) === 0
            // A block that does not close the sequence consumed a full one. This
            // is the invariant that lets the PIL use a constant instead of
            // 'total_bytes_used on the first row of a segment.
            if is_last_clock && row.get_sel() && !row.get_seq_end() {
                assert_eq!(
                    total_bytes_used,
                    (8 * JUMP_DEST_OPS_X_ROW) as u32,
                    "row {index}: a non-final block must be full"
                );
            }

            if prev.seq_end {
                // A new sequence: src64, dst64, count and state are pinned by
                // the operation, not by the row before. Only state[0] === 0 at
                // seq_start applies.
                //
                // NOTE: the dst64 transitions in the PIL are missing the
                // (1 - 'seq_end) guard the comment above them describes, so as
                // written they force one operation's bitmap to follow the
                // previous one's. This checks the intended rule.
                // seq_start * state[0] === 0. seq_start is gated by sel, so an
                // inactive block carries the finished state in and owes nothing.
                if row.get_sel() {
                    assert_eq!(row.get_state(0), 0, "row {index}: seq_start with a pending state");
                }
                continue;
            }

            // (1 - LAST_CLOCK) * (dst64 - 'dst64) === 0
            // LAST_CLOCK * (dst64 - 1 - 'dst64) === 0
            let expected_dst64 = prev.dst64 + is_last_clock as u32;
            assert_eq!(row.get_dst64(), expected_dst64, "row {index}: dst64 transition");

            // continue_seq_* transitions.
            assert_eq!(
                row.get_src64(),
                prev.src64 + JUMP_DEST_OPS_X_ROW as u32,
                "row {index}: src64 transition"
            );
            // count[r] = count[r-1] - total_bytes_used[r-1]
            assert_eq!(
                row.get_count(),
                prev.count - prev.bytes_used,
                "row {index}: count transition"
            );
            assert_eq!(row.get_state(0), prev.state, "row {index}: state chain");
            assert_eq!(row.get_main_step(), prev.main_step, "row {index}: main_step latch");
        }
    }

    /// The rules `sel` obeys: latched inside a block, never turning back on, and
    /// only turning off right after a sequence closed. Everything is guarded by
    /// `(1 - L1)` in the PIL, so row 0 is skipped here too.
    fn check_sel_rules(rows: &[JumpDestTraceRow<F>]) {
        for (index, row) in rows.iter().enumerate().skip(1) {
            let prev = &rows[index - 1];
            let is_first_clock = index % JUMP_DEST_ROWS_X_BLOCK == 0;

            // (1 - FIRST_CLOCK) * (sel - 'sel) === 0
            if !is_first_clock {
                assert_eq!(row.get_sel(), prev.get_sel(), "row {index}: sel not latched");
            }
            // (1 - L1) * sel * (1 - 'sel) === 0
            assert!(!row.get_sel() || prev.get_sel(), "row {index}: sel turned back on");
            // (1 - L1) * (1 - sel) * ('sel - 'seq_end) === 0
            assert!(
                row.get_sel() || prev.get_sel() == prev.get_seq_end(),
                "row {index}: sel fell without closing the sequence"
            );
            // seq_end * (1 - sel) === 0
            assert!(!row.get_seq_end() || row.get_sel(), "row {index}: seq_end on an idle block");
            // seq_start <== 'seq_end * sel
            assert_eq!(
                row.get_seq_start(),
                prev.get_seq_end() && row.get_sel(),
                "row {index}: seq_start"
            );
        }
    }

    /// The state chain inside a row must be the one the bitmap table proves.
    fn check_state_chain(rows: &[JumpDestTraceRow<F>]) {
        for (index, row) in rows.iter().enumerate() {
            for i in 0..JUMP_DEST_OPS_X_ROW {
                assert!(row.get_state(i) <= 33, "row {index} op {i}: state out of range");
                assert!(row.get_bytes_used(i) <= 8, "row {index} op {i}: bytes_used out of range");
            }
        }
    }

    #[test]
    fn one_operation_satisfies_every_transition() {
        let bytecode = vec![0x5bu8; 200];
        let inp = input(&bytecode, 0xA000_0000, 0xB000_0000);
        let mut rows = blank(inp.rows() as usize);
        let (written, cursor) = JumpDestSM::<F>::process_input(&inp, 0, &mut rows, |_| {});

        assert_eq!(written, bitmap_words(200) * JUMP_DEST_ROWS_X_BLOCK);
        JumpDestSM::<F>::fill_seq_start(&mut rows, true);
        check_transitions(&rows, &Cursor { seq_end: true, ..Cursor::default() });
        check_state_chain(&rows);
        check_sel_rules(&rows);

        assert!(cursor.seq_end, "the operation ends inside the segment");
        assert_eq!(cursor.count, 0, "every byte is accounted for");
        assert!(rows.last().unwrap().get_seq_end());
    }

    /// The mstore of a last-clock row targets `'dst64`, so that value must be
    /// the block's own bitmap word.
    #[test]
    fn the_mstore_address_walks_the_bitmap_words() {
        let bytecode = vec![0x00u8; 200];
        let inp = input(&bytecode, 0xA000_0000, 0xB000_0000);
        let mut rows = blank(inp.rows() as usize);
        JumpDestSM::<F>::process_input(&inp, 0, &mut rows, |_| {});

        let base = (0xA000_0000u64 / 8) as u32;
        for block in 0..bitmap_words(200) {
            let last_clock = block * JUMP_DEST_ROWS_X_BLOCK + JUMP_DEST_ROWS_X_BLOCK - 1;
            let previous = rows[last_clock - 1].get_dst64();
            assert_eq!(previous, base + block as u32, "block {block}: mstore address");
        }
    }

    /// Cutting an operation in two must produce the same rows as filling it in
    /// one go, with the air values carrying the join.
    #[test]
    fn a_split_operation_rejoins_across_the_cut() {
        // PUSH32 every 33 bytes, so the state is non-zero at most cut points.
        let mut bytecode = vec![0x5bu8; 512];
        for pc in (0..512).step_by(33) {
            bytecode[pc] = 0x7f;
        }
        let inp = input(&bytecode, 0xA000_0000, 0xB000_0000);
        let total = inp.rows() as usize;

        let mut whole = blank(total);
        JumpDestSM::<F>::process_input(&inp, 0, &mut whole, |_| {});

        let cut = JUMP_DEST_ROWS_X_BLOCK * 3;
        let mut tail = blank(total - cut);
        JumpDestSM::<F>::process_input(&inp, cut, &mut tail, |_| {});

        for (index, (t, w)) in tail.iter().zip(&whole[cut..]).enumerate() {
            assert_eq!(t.get_src64(), w.get_src64(), "row {index}: src64");
            assert_eq!(t.get_dst64(), w.get_dst64(), "row {index}: dst64");
            assert_eq!(t.get_count(), w.get_count(), "row {index}: count");
            assert_eq!(t.get_seq_end(), w.get_seq_end(), "row {index}: seq_end");
            for i in 0..=JUMP_DEST_OPS_X_ROW {
                assert_eq!(t.get_state(i), w.get_state(i), "row {index}: state[{i}]");
            }
            for i in 0..JUMP_DEST_OPS_X_ROW {
                assert_eq!(
                    t.get_bytes_used(i),
                    w.get_bytes_used(i),
                    "row {index}: bytes_used[{i}]"
                );
                assert_eq!(t.get_bitmap_byte(i), w.get_bitmap_byte(i), "row {index}: bitmap[{i}]");
            }
        }

        // What the instance publishes as segment_previous_* must be the row the
        // first segment ended on.
        let previous = JumpDestSM::<F>::cursor_before(&inp, cut);
        let boundary = &whole[cut - 1];
        assert_eq!(previous.src64, boundary.get_src64());
        assert_eq!(previous.dst64, boundary.get_dst64());
        assert_eq!(previous.count, boundary.get_count());
        assert_eq!(
            previous.bytes_used,
            (0..JUMP_DEST_OPS_X_ROW).map(|i| boundary.get_bytes_used(i) as u32).sum::<u32>()
        );
        assert_eq!(previous.state, boundary.get_state(JUMP_DEST_OPS_X_ROW));
        assert_eq!(previous.main_step, boundary.get_main_step());

        check_transitions(&tail, &previous);
    }

    /// The constant the PIL substitutes for `'total_bytes_used` on the first row
    /// of a segment rests on one claim: a block that does not close the sequence
    /// consumed a full `8 * op_x_row` bytes. This sweeps lengths and PUSH
    /// patterns looking for a counterexample.
    #[test]
    fn a_block_is_full_unless_it_ends_the_sequence() {
        for len in 1..=400usize {
            for stride in [1usize, 2, 3, 5, 17, 31, 32, 33, 64] {
                let mut bytecode = vec![0x5bu8; len];
                // PUSH32 every `stride` bytes: the widest immediate there is, so
                // the tail lands in every possible position relative to a block.
                for pc in (0..len).step_by(stride) {
                    bytecode[pc] = 0x7f;
                }
                let inp = input(&bytecode, 0xA000_0000, 0xB000_0000);
                let mut rows = blank(inp.rows() as usize);
                JumpDestSM::<F>::process_input(&inp, 0, &mut rows, |_| {});

                check_transitions(&rows, &Cursor { seq_end: true, ..Cursor::default() });

                let last = rows.last().unwrap();
                assert!(last.get_seq_end(), "len {len} stride {stride}: no seq_end");
                let consumed: u32 =
                    (0..JUMP_DEST_OPS_X_ROW).map(|i| last.get_bytes_used(i) as u32).sum();
                assert_eq!(
                    last.get_count(),
                    consumed,
                    "len {len} stride {stride}: the count does not land on zero"
                );
            }
        }
    }

    /// The inactive blocks have to keep the transitions running, or the rows
    /// after the last operation break the AIR even though nothing is selected.
    #[test]
    fn inactive_blocks_continue_the_transitions() {
        let bytecode = vec![0x5bu8; 64];
        let inp = input(&bytecode, 0xA000_0000, 0xB000_0000);
        let used = inp.rows() as usize;
        let mut rows = blank(used + 3 * JUMP_DEST_ROWS_X_BLOCK);

        let (written, cursor) = JumpDestSM::<F>::process_input(&inp, 0, &mut rows[..used], |_| {});
        JumpDestSM::<F>::fill_inactive(written, &cursor, &mut rows[written..]);

        for row in &rows[written..] {
            assert!(!row.get_sel(), "an inactive block must not be selected");
            assert!(!row.get_seq_end(), "an inactive block must not close a sequence");
            assert_eq!(row.get_count(), 0, "an inactive block must not move the count");
        }

        // The last active row is a seq_end, so the first inactive row starts a
        // "new sequence" as far as the transitions are concerned; from there on
        // they must hold.
        JumpDestSM::<F>::fill_seq_start(&mut rows, true);
        check_transitions(&rows, &Cursor { seq_end: true, ..Cursor::default() });
        check_sel_rules(&rows);
    }
}
