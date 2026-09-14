//! Writing one lane of a `CompactMemAlign` row.
//!
//! The fused air carries the two mem-align airs side by side on every row (see
//! `state-machines/mem/pil/compact_mem_align.pil`), each one packing `lanes_x_row` of its rows into
//! one row of the fused air. The witness of each block is computed exactly as it is for the air it
//! comes from -- what differs is only where the result is written: into the `full_*` / `bytes_*`
//! columns of a lane instead of into a row of its own.
//!
//! These are the writers for that, and the only place the lane packing shows up in the fill.
//!
//! # Why a trait and not the generated row ops
//!
//! The small and the tall air are instantiated with different lane counts, so they do NOT commit
//! the same columns and each gets a generated row type of its own. The fills are written once and
//! instantiated for whichever row they are filling, and the traits below are that difference and
//! nothing else. They are deliberately NOT blanket-implemented over the generated `*TraceRowOps`
//! traits: there is one such trait per air, and two blanket impls would collide because coherence
//! cannot see that a row type does not implement both.
//!
//! There is one trait per block on purpose: a fill can then only reach the block it owns, which is
//! what makes running the two over the same rows sound. Each writer sets EVERY column of its
//! block's lane -- the trace buffer comes from the recycled pool and is not zeroed, so a column
//! left alone would keep whatever the previous instance wrote into it.

use proofman_fields::{Goldilocks, PrimeField64};
use zisk_pil::{
    CompactMemAlignLargeTraceRow, CompactMemAlignLargeTraceRowPacked, CompactMemAlignTraceRow,
    CompactMemAlignTraceRowPacked, MemAlignTraceRow,
};

use crate::ByteRowValues;

/// The `full_` block of a fused row: the lanes of the `MemAlign` sequence it carries.
pub(crate) trait MemAlignFullBlockRow<F: PrimeField64>:
    Default + Copy + Send + Sync + 'static
{
    /// Lanes of the sequence one row carries, read from the generated row so the Rust side always
    /// follows `full_lanes_x_row` in the PIL.
    fn full_lanes_x_row() -> usize;

    /// Writes the `MemAlign` row `src` into the lane `lane`, leaving every other block alone.
    ///
    /// `full_delta_addr` is left out on purpose: it is the air's `<==` column, derived by the
    /// prover before it commits, and the standalone fill does not write it either.
    fn set_full_lane(&mut self, lane: usize, src: &MemAlignTraceRow<F>);
}

/// The `bytes_` block of a fused row: the lanes of the `MemAlignByte` sequence it carries.
pub(crate) trait MemAlignBytesBlockRow<F: PrimeField64>:
    Default + Copy + Send + Sync + 'static
{
    /// Lanes of the sequence one row carries, read from the generated row so the Rust side always
    /// follows `bytes_lanes_x_row` in the PIL.
    fn bytes_lanes_x_row() -> usize;

    /// Writes one byte row into the lane `lane`, leaving every other block alone.
    fn set_bytes_lane(&mut self, lane: usize, src: &ByteRowValues);
}

/// The two writers for one generated row type. The column names are the same in every air of the
/// family -- only the lane counts differ -- so one body serves them all.
macro_rules! impl_compact_mem_align_row {
    ($row:ident) => {
        impl<F: PrimeField64> MemAlignFullBlockRow<F> for $row<F> {
            #[inline(always)]
            fn full_lanes_x_row() -> usize {
                <$row<Goldilocks> as Default>::default().get_all_full_addr().len()
            }

            #[inline(always)]
            fn set_full_lane(&mut self, lane: usize, src: &MemAlignTraceRow<F>) {
                self.set_full_addr(lane, src.get_addr());
                self.set_full_offset(lane, src.get_offset());
                self.set_full_width(lane, src.get_width());
                self.set_full_wr(lane, src.get_wr());
                self.set_full_pc(lane, src.get_pc());
                self.set_full_reset(lane, src.get_reset());
                self.set_full_sel_up_to_down(lane, src.get_sel_up_to_down());
                self.set_full_sel_down_to_up(lane, src.get_sel_down_to_up());
                self.set_full_is_non_aligned_op(lane, src.get_is_non_aligned_op());
                self.set_full_sel_w_lt8(lane, src.get_sel_w_lt8());
                self.set_full_sel_w_lt4(lane, src.get_sel_w_lt4());
                self.set_full_sel_w_lt2(lane, src.get_sel_w_lt2());
                self.set_full_step(lane, src.get_step());

                let reg = src.get_all_reg();
                let sel = src.get_all_sel();
                for i in 0..reg.len() {
                    self.set_full_reg(lane, i, reg[i]);
                    self.set_full_sel(lane, i, sel[i]);
                }

                let value = src.get_all_value();
                for (i, v) in value.iter().enumerate() {
                    self.set_full_value(lane, i, *v);
                }
            }
        }

        impl<F: PrimeField64> MemAlignBytesBlockRow<F> for $row<F> {
            #[inline(always)]
            fn bytes_lanes_x_row() -> usize {
                <$row<Goldilocks> as Default>::default().get_all_bytes_addr_w().len()
            }

            #[inline(always)]
            fn set_bytes_lane(&mut self, lane: usize, src: &ByteRowValues) {
                self.set_bytes_sel_high_4b(lane, src.sel_high_4b);
                self.set_bytes_sel_high_2b(lane, src.sel_high_2b);
                self.set_bytes_sel_high_b(lane, src.sel_high_b);
                self.set_bytes_direct_value(lane, src.direct_value);
                self.set_bytes_composed_value(lane, src.composed_value);
                self.set_bytes_value_16b(lane, src.value_16b);
                self.set_bytes_value_8b(lane, src.value_8b);
                self.set_bytes_byte_value(lane, src.byte_value);
                self.set_bytes_addr_w(lane, src.addr_w);
                self.set_bytes_step(lane, src.step);

                // The block is the read+write air, so it carries the write columns whatever the
                // operation is: on a read they are the ones a `MemAlignByte` read row commits.
                self.set_bytes_is_write(lane, src.is_write);
                self.set_bytes_written_composed_value(lane, src.written_composed_value);
                self.set_bytes_written_byte_value(lane, src.written_byte_value);
                self.set_bytes_mem_write_values(lane, 0, src.mem_write_values[0]);
                self.set_bytes_mem_write_values(lane, 1, src.mem_write_values[1]);
                self.set_bytes_bus_byte(
                    lane,
                    if src.is_write { src.written_byte_value } else { src.byte_value },
                );
            }
        }
    };
}

impl_compact_mem_align_row!(CompactMemAlignTraceRow);
impl_compact_mem_align_row!(CompactMemAlignTraceRowPacked);
impl_compact_mem_align_row!(CompactMemAlignLargeTraceRow);
impl_compact_mem_align_row!(CompactMemAlignLargeTraceRowPacked);
