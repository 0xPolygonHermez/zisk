//! The `DmaCompactSM` computes the witness of the `DmaCompact` AIR: one small instance that
//! proves whole DMA operations (controller, partial writes and loop) by itself. See
//! `precompiles/dma/pil/dma_compact.pil`.

use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use precompiles_helpers::DmaInfo;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use rayon::prelude::*;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{
    DmaCompactTrace, DmaCompactTraceRow, DmaCompactTraceRowOps, DmaCompactTraceRowPacked,
    DMA_COMPACT_TABLE_ID, DMA_COMPACT_TABLE_SIZE, DMA_ROM_ID, DUAL_RANGE_BYTE_ID,
};

use crate::{
    dma_trace, DmaCompactInput, DmaCompactLayout, DmaCompactRow, DmaCompactTable, DmaRom,
    DMA_ROM_WITH_MEMCMP_SIZE,
};

/// Dual byte range checks that EVERY row of the AIR does: 8 pairs of the window, 4 pairs of the
/// previous destination value and the pair (0, abs_diff).
const DUAL_BYTE_PAIRS_X_ROW: u64 = 13;

/// Values of the range check of `abs_diff - 1`: [0, 254].
const ABS_DIFF_RANGE_SIZE: usize = 255;

/// Multiplicities a group of rows contributes to the tables of the AIR.
struct LocalMultiplicities {
    rom: Vec<u64>,
    table: Vec<u64>,
    dual_byte: Vec<u64>,
    abs_diff: Vec<u64>,
}

impl LocalMultiplicities {
    fn new() -> Self {
        Self {
            rom: vec![0; DMA_ROM_WITH_MEMCMP_SIZE],
            table: vec![0; DMA_COMPACT_TABLE_SIZE],
            dual_byte: vec![0; 1 << 16],
            abs_diff: vec![0; ABS_DIFF_RANGE_SIZE],
        }
    }

    fn merge(&mut self, other: &Self) {
        for (acc, val) in self.rom.iter_mut().zip(&other.rom) {
            *acc += val;
        }
        for (acc, val) in self.table.iter_mut().zip(&other.table) {
            *acc += val;
        }
        for (acc, val) in self.dual_byte.iter_mut().zip(&other.dual_byte) {
            *acc += val;
        }
        for (acc, val) in self.abs_diff.iter_mut().zip(&other.abs_diff) {
            *acc += val;
        }
    }
}

/// The `DmaCompactSM` struct encapsulates the logic of the DmaCompact State Machine.
pub struct DmaCompactSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// DmaRom, consulted by the controller rows.
    rom_table_id: usize,

    /// DmaCompactTable, consulted by the pre/post rows.
    compact_table_id: usize,

    /// Dual byte range checks of the byte pool.
    dual_range_byte_id: usize,

    /// Range check of `abs_diff - 1` in [0, 254] (a memcmp difference is never zero).
    abs_diff_range_id: usize,
}

impl<F: PrimeField64> DmaCompactSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            rom_table_id: std
                .get_virtual_table_id(DMA_ROM_ID)
                .expect("Failed to get table DMA_ROM identifier"),
            compact_table_id: std
                .get_virtual_table_id(DMA_COMPACT_TABLE_ID)
                .expect("Failed to get table DMA_COMPACT_TABLE identifier"),
            dual_range_byte_id: std
                .get_virtual_table_id(DUAL_RANGE_BYTE_ID)
                .expect("Failed to get table DUAL_RANGE_BYTE identifier"),
            abs_diff_range_id: std
                .get_range_id(0, ABS_DIFF_RANGE_SIZE as i64 - 1, None)
                .expect("Failed to get range [0, 254] identifier"),
        })
    }

    /// Copies a row into the trace, column by column.
    fn fill_row<R: DmaCompactTraceRowOps<F>>(row: &DmaCompactRow<F>, trace: &mut R) {
        trace.set_is_ctrl(row.is_ctrl);
        trace.set_is_pp(row.is_pp);
        trace.set_is_loop(row.is_loop);
        trace.set_sel_memcpy(row.sel_memcpy);
        trace.set_sel_memcmp(row.sel_memcmp);
        trace.set_sel_inputcpy(row.sel_inputcpy);
        trace.set_sel_memset(row.sel_memset);
        trace.set_all_rb(&row.rb);
        trace.set_all_pb(&row.pb);
        trace.set_abs_diff(row.abs_diff);
        trace.set_all_sb(&row.sb);
        trace.set_all_sr(&row.sr);
        trace.set_pp_flag_a(row.pp_flag_a);
        trace.set_borrow(row.borrow);
        trace.set_read_lo(row.read_lo);
        trace.set_read_hi(row.read_hi);
        trace.set_memcmp_nz(row.memcmp_nz);
        trace.set_memcmp_neg(row.memcmp_neg);
        trace.set_load_count(row.load_count);
        trace.set_main_step(row.main_step);
        trace.set_dst64(row.dst64 as u32);
        trace.set_src64(row.src64 as u32);
        trace.set_dst_offset(row.dst_offset);
        trace.set_src_offset(row.src_offset);
        trace.set_fill_byte(row.fill_byte);
        trace.set_all_wv(&[row.wv[0] as u32, row.wv[1] as u32]);
        trace.set_cnt_small(row.cnt_small);
        trace.set_op_col(row.op_col as u8);
        trace.set_b0_col(row.b0_col as u32);
        trace.set_c0_col(row.c0_col as u32);
        trace.set_ext_col(row.ext_col as u32);
        trace.set_xa_col(row.xa_col as u8);
        trace.set_b0_loop(row.b0_loop as u32);
        trace.set_ext_loop(row.ext_loop as u32);
        trace.set_x_count(row.x_count as u32);
        trace.set_count(row.count as u32);
        trace.set_rom_l_count64(row.rom_l_count64 as u16);
        trace.set_pp_cmp(row.pp_cmp);
        trace.set_sel_w2(row.sel_w2);
        trace.set_sel_w34(row.sel_w34);
        trace.set_rd_34(row.rd_34);
        trace.set_loop_end(row.loop_end);
        trace.set_wr_loop(row.wr_loop);
        trace.set_all_v3(&[row.v3[0] as u32, row.v3[1] as u32]);
        trace.set_all_v4(&[row.v4[0] as u32, row.v4[1] as u32]);
        trace.set_all_rv(&[row.rv[0] as u32, row.rv[1] as u32]);
        trace.set_all_pw(&[row.pw[0].as_canonical_u64(), row.pw[1].as_canonical_u64()]);
    }

    /// The range checks every used row does, whatever its kind.
    #[inline(always)]
    fn account_row(row: &DmaCompactRow<F>, muls: &mut LocalMultiplicities) {
        // range_dual_byte(high, low): the table row is the little endian 16-bit value
        for pair in row.rb.chunks_exact(2) {
            muls.dual_byte[pair[0] as usize | (pair[1] as usize) << 8] += 1;
        }
        for pair in row.pb.chunks_exact(2) {
            muls.dual_byte[pair[0] as usize | (pair[1] as usize) << 8] += 1;
        }
        muls.dual_byte[row.abs_diff as usize] += 1;
        if row.memcmp_nz {
            debug_assert!(row.abs_diff > 0);
            muls.abs_diff[row.abs_diff as usize - 1] += 1;
        }
    }

    /// Writes the rows of one operation from `rows[0]` on, returning how many it used.
    fn process_input<R: DmaCompactTraceRowOps<F>>(
        &self,
        input: &DmaCompactInput,
        rows: &mut [R],
        muls: &mut LocalMultiplicities,
    ) -> usize {
        let encoded = input.encoded;
        let layout = input.layout();
        let has_src = DmaCompactLayout::has_src(input.op);
        let fill_byte =
            if input.op == ZiskOp::DMA_XMEMSET { DmaInfo::get_fill_byte(encoded) } else { 0 };
        let count = DmaInfo::get_count(encoded);
        let pre_count = DmaInfo::get_pre_count(encoded);
        let post_count = DmaInfo::get_post_count(encoded);
        let (pre_result, post_result) = input.phase_results();

        let mut used = 0;

        // controller row
        let ctrl = DmaCompactRow::<F>::ctrl(
            input.op,
            input.dst,
            input.src,
            input.step,
            encoded,
            input.count_bus,
            pre_result,
            post_result,
        );
        let result_nz = input.is_memcmp() && DmaInfo::get_memcmp_result_nz(encoded);
        muls.rom[DmaRom::get_row(input.dst & 0x07, input.src & 0x07, count, result_nz, has_src)] +=
            1;
        Self::account_row(&ctrl, muls);
        Self::fill_row(&ctrl, &mut rows[used]);
        used += 1;

        // pre row
        if pre_count > 0 {
            let row = DmaCompactRow::<F>::pre_post(
                input.op,
                false,
                input.dst as u64,
                input.src as u64,
                input.step,
                pre_count,
                input.pre_src_values,
                input.pre_dst_value,
                fill_byte,
                input.memcmp_row(false),
            );
            muls.table[DmaCompactTable::get_row(
                row.dst_offset as usize,
                row.src_offset as usize,
                pre_count,
                has_src,
                row.memcmp_nz,
                row.memcmp_neg,
            )] += 1;
            Self::account_row(&row, muls);
            Self::fill_row(&row, &mut rows[used]);
            used += 1;
        }

        // loop rows: the controller delegates from the word after the pre phase
        if layout.loop_words > 0 {
            let loop_dst64 = (input.dst as u64 + pre_count as u64) >> 3;
            let loop_src64 = if has_src { (input.src as u64 + pre_count as u64) >> 3 } else { 0 };
            let loop_rows = DmaCompactRow::<F>::loop_sequence(
                input.op,
                loop_dst64,
                loop_src64,
                input.step,
                layout.loop_words,
                layout.delta as usize,
                &input.loop_values,
                fill_byte,
            );
            debug_assert_eq!(loop_rows.len(), layout.loop_rows);
            for row in &loop_rows {
                Self::account_row(row, muls);
                Self::fill_row(row, &mut rows[used]);
                used += 1;
            }
        }

        // post row. Without a source the controller still advances the (zero) source by the
        // bytes done, exactly as Dma and DmaPrePost do, so the row must say the same.
        if post_count > 0 {
            let done = pre_count as u64 + layout.loop_words as u64 * 8;
            let dst = input.dst as u64 + done;
            let src = if has_src { input.src as u64 + done } else { done };
            let row = DmaCompactRow::<F>::pre_post(
                input.op,
                true,
                dst,
                src,
                input.step,
                post_count,
                input.post_src_values,
                input.post_dst_value,
                fill_byte,
                input.memcmp_row(true),
            );
            muls.table[DmaCompactTable::get_row(
                0,
                row.src_offset as usize,
                post_count,
                has_src,
                row.memcmp_nz,
                row.memcmp_neg,
            )] += 1;
            Self::account_row(&row, muls);
            Self::fill_row(&row, &mut rows[used]);
            used += 1;
        }

        debug_assert_eq!(used, layout.rows(), "rows of the operation differ from its layout");
        used
    }

    fn compute_witness_inner<R: DmaCompactTraceRowOps<F>>(
        &self,
        inputs: &[Vec<DmaCompactInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaCompactTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        timer_start_trace!(DMA_COMPACT_TRACE);

        // Operations are independent (no continuations), so they are laid out one after another in
        // the order the collectors gave them, and groups of operations are filled in parallel.
        let flat_inputs: Vec<&DmaCompactInput> = inputs.iter().flatten().collect();
        let rows_x_input: Vec<usize> = flat_inputs.iter().map(|input| input.rows()).collect();
        let total_rows: usize = rows_x_input.iter().sum();
        assert!(total_rows <= num_rows, "DmaCompact: {total_rows} rows do not fit in {num_rows}");

        dma_trace("DmaCompact", total_rows, num_rows);

        // groups of consecutive operations with about the same number of rows
        let target_rows = total_rows.div_ceil(rayon::current_num_threads().max(1)).max(1);
        let mut groups: Vec<(usize, usize, usize)> = Vec::new(); // (first input, inputs, rows)
        let (mut first, mut acc_rows) = (0, 0);
        for (index, rows) in rows_x_input.iter().enumerate() {
            acc_rows += rows;
            if acc_rows >= target_rows {
                groups.push((first, index + 1 - first, acc_rows));
                first = index + 1;
                acc_rows = 0;
            }
        }
        if first < flat_inputs.len() {
            groups.push((first, flat_inputs.len() - first, acc_rows));
        }

        let (used_rows, padding_rows) = trace.buffer.as_mut_slice().split_at_mut(total_rows);
        let mut slices = Vec::with_capacity(groups.len());
        let mut rest = used_rows;
        for (_, _, rows) in &groups {
            let (slice, tail) = rest.split_at_mut(*rows);
            slices.push(slice);
            rest = tail;
        }

        let mut muls = groups
            .par_iter()
            .zip(slices.into_par_iter())
            .map(|((first, count, rows), slice)| {
                let mut local = LocalMultiplicities::new();
                let mut offset = 0;
                for input in &flat_inputs[*first..*first + *count] {
                    offset += self.process_input(input, &mut slice[offset..], &mut local);
                }
                debug_assert_eq!(offset, *rows);
                local
            })
            .reduce(LocalMultiplicities::new, |mut acc, local| {
                acc.merge(&local);
                acc
            });

        // padding: every column at zero but seq_end, which every row that is not a loop row has
        padding_rows.par_iter_mut().for_each(|row| row.set_pp_flag_a(true));
        muls.dual_byte[0] += DUAL_BYTE_PAIRS_X_ROW * padding_rows.len() as u64;

        self.std.inc_virtual_rows_ranged(self.rom_table_id, None, &muls.rom);
        self.std.inc_virtual_rows_ranged(self.compact_table_id, None, &muls.table);
        self.std.inc_virtual_rows_ranged(self.dual_range_byte_id, None, &muls.dual_byte);
        self.std.range_check_ranged(self.abs_diff_range_id, None, &muls.abs_diff);

        timer_stop_and_log_trace!(DMA_COMPACT_TRACE);

        let from_trace = FromTrace::new(&mut trace);
        Ok(AirInstance::new_from_trace(from_trace))
    }

    pub fn compute_witness(
        &self,
        inputs: &[Vec<DmaCompactInput>],
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<DmaCompactTraceRowPacked<F>>(inputs, trace_buffer)
        } else {
            self.compute_witness_inner::<DmaCompactTraceRow<F>>(inputs, trace_buffer)
        }
    }
}
