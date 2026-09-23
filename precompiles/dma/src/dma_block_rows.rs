//! One fill, two traces: the row views the DMA fills write through.
//!
//! `DmaWithPrePost` and `DmaLoop` are each proved by two airs -- their own and the fused
//! `CompactDma`, which carries the two of them side by side on every row (see
//! `precompiles/dma/pil/compact_dma.pil`). The witness of a block is built the same way in both,
//! but the generated row types are different: `DmaLoopTraceRow` names its columns `count`,
//! `seq_end`, ... while `CompactDmaTraceRow` names the same columns `loop_count`, `loop_seq_end`,
//! ... alongside the `wpp_*` block.
//!
//! The traits below are that difference and nothing else: one method per column the fill writes,
//! implemented for the standalone rows and for the fused one, so each fill is written once and
//! instantiated for whichever row it is filling. They are deliberately NOT derived from the
//! generated `*TraceRowOps` traits -- a blanket impl over those would collide with the
//! `CompactDma` impls, because coherence cannot see that `CompactDmaTraceRow` does not implement
//! them. There is one trait per block on purpose: a fill can then only reach the block it owns,
//! which is what makes running the two over the same rows sound.
//!
//! The one place that writes a row wholesale -- the bulk padding of the `DmaLoop` fill -- goes
//! through `copy_block_from`, which copies the block and nothing else.
//!
//! The columns an air defines with `<==` are not in the traits: the prover derives them from their
//! expression before it commits, and the fills never write them.

use proofman_common::trace::TraceRow;
use proofman_fields::PrimeField64;
use zisk_pil::{
    CompactDmaTraceRow, CompactDmaTraceRowPacked, DmaLoopTraceRow, DmaLoopTraceRowPacked,
    DmaWithPrePostTraceRow, DmaWithPrePostTraceRowPacked,
};

use crate::DMA_LOOP_OPS_BY_ROW;

/// Declares a block trait from its field list.
macro_rules! block_trait {
    (
        $(#[$meta:meta])*
        $trait:ident {
            scalars { $($s:ident: $st:ty),* $(,)? }
            arrays { $($a:ident: $at:ty),* $(,)? }
        }
    ) => {
        paste::paste! {
            $(#[$meta])*
            pub trait $trait<F: PrimeField64>:
                TraceRow + Default + Copy + Send + Sync + 'static
            {
                $(
                    fn [<set_ $s>](&mut self, value: $st);
                    fn [<get_ $s>](&self) -> $st;
                )*
                $(
                    fn [<set_all_ $a>](&mut self, values: &$at);
                    fn [<get_all_ $a>](&self) -> $at;
                )*

                /// Overwrites this block of the row with `src`'s, leaving every other block
                /// untouched. The padding of a segment is one row repeated, built once and copied:
                /// on a standalone row the block IS the row and this is a plain assignment, but on
                /// a `CompactDma` row it must not take the other block's columns with it -- those
                /// belong to a fill that has not run yet, or has already run.
                fn copy_block_from(&mut self, src: &Self);
            }
        }
    };
}

/// Implements a block trait for one generated row, forwarding every method to the row's own
/// column, which carries `prefix` when the block lives inside a fused row.
macro_rules! block_impl {
    ($trait:ident for $row:ident {
        scalars { $($s:ident: $st:ty),* $(,)? }
        arrays { $($a:ident: $at:ty),* $(,)? }
    }) => {
        paste::paste! {
            impl<F: PrimeField64> $trait<F> for $row<F> {
                $(
                    #[inline(always)]
                    fn [<set_ $s>](&mut self, value: $st) { $row::<F>::[<set_ $s>](self, value) }
                    #[inline(always)]
                    fn [<get_ $s>](&self) -> $st { $row::<F>::[<get_ $s>](self) }
                )*
                $(
                    #[inline(always)]
                    fn [<set_all_ $a>](&mut self, values: &$at) {
                        $row::<F>::[<set_all_ $a>](self, values)
                    }
                    #[inline(always)]
                    fn [<get_all_ $a>](&self) -> $at { $row::<F>::[<get_all_ $a>](self) }
                )*
                #[inline(always)]
                fn copy_block_from(&mut self, src: &Self) {
                    *self = *src;
                }
            }
        }
    };
    ($trait:ident for $row:ident prefix $prefix:ident {
        scalars { $($s:ident: $st:ty),* $(,)? }
        arrays { $($a:ident: $at:ty),* $(,)? }
    }) => {
        paste::paste! {
            impl<F: PrimeField64> $trait<F> for $row<F> {
                $(
                    #[inline(always)]
                    fn [<set_ $s>](&mut self, value: $st) {
                        $row::<F>::[<set_ $prefix $s>](self, value)
                    }
                    #[inline(always)]
                    fn [<get_ $s>](&self) -> $st { $row::<F>::[<get_ $prefix $s>](self) }
                )*
                $(
                    #[inline(always)]
                    fn [<set_all_ $a>](&mut self, values: &$at) {
                        $row::<F>::[<set_all_ $prefix $a>](self, values)
                    }
                    #[inline(always)]
                    fn [<get_all_ $a>](&self) -> $at {
                        $row::<F>::[<get_all_ $prefix $a>](self)
                    }
                )*
                #[inline(always)]
                fn copy_block_from(&mut self, src: &Self) {
                    $( $row::<F>::[<set_ $prefix $s>](self, $row::<F>::[<get_ $prefix $s>](src)); )*
                    $(
                        $row::<F>::[<set_all_ $prefix $a>](
                            self,
                            &$row::<F>::[<get_all_ $prefix $a>](src),
                        );
                    )*
                }
            }
        }
    };
}

/// The columns of the `DmaWithPrePost` block, handed to `$m` after the tokens given.
macro_rules! with_wpp_fields {
    ($m:ident! { $($head:tt)* }) => {
        $m! { $($head)* {
            scalars {
                is_pre_row: bool,
                sel_memcpy: bool,
                sel_memcmp: bool,
                sel_memset: bool,
                sel_inputcpy: bool,
                sel_extended: bool,
                fill_byte: u8,
                h_count: u32,
                count_lt_256: bool,
                l_count: u16,
                main_step: u64,
                h_dst64: u32,
                l_dst64: u8,
                dst_offset: u8,
                h_src64: u32,
                l_src64: u8,
                src_offset: u8,
                src_offset_after_pre: u8,
                src64_inc_by_pre: bool,
                use_pre: bool,
                use_loop: bool,
                use_post: bool,
                pre_count: u8,
                l_count64: u16,
                dst_offset_gt_src_offset: bool,
                enabled_second_read: bool,
                memcmp_result_nz: bool,
                abs_diff_dst_src: u8,
                memcmp_result_is_negative: bool,
            }
            arrays {
                count_diff_chunks: [u16; 2],
                selr: [bool; 7],
                rb: [u8; 16],
                pb: [u8; 8],
                sb: [bool; 8],
                diff_factor: [u64; 2],
            }
        }}
    };
}

/// The columns of the `DmaLoop` block, handed to `$m` after the tokens given.
macro_rules! with_loop_fields {
    ($m:ident! { $($head:tt)* }) => {
        $m! { $($head)* {
            scalars {
                main_step: u64,
                src64: u32,
                dst64: u32,
                count: u32,
                seq_end: bool,
                sel_memcpy: bool,
                sel_memeq: bool,
                sel_memset: bool,
                sel_inputcpy: bool,
                sel_memcpy_count_load: bool,
                fill_byte: u8,
                offset_1: bool,
                offset_2: bool,
                offset_3: bool,
                offset_4: bool,
                offset_5: bool,
                offset_6: bool,
                offset_7: bool,
            }
            arrays {
                sel_op_from_1: [bool; DMA_LOOP_OPS_BY_ROW - 1],
                read_bytes: [u8; 8 * DMA_LOOP_OPS_BY_ROW],
            }
        }}
    };
}

with_wpp_fields!(block_trait! {
    /// The columns of a `DmaWithPrePost` row, as `DmaWithPrePostSM` writes them.
    ///
    /// A row the fill does not reach is left as the trace buffer holds it, which is zero: both
    /// airs are built on a zeroed buffer, and an all-zero row is the air's padding.
    DmaWithPrePostBlockRow
});
with_wpp_fields!(block_impl! { DmaWithPrePostBlockRow for DmaWithPrePostTraceRow });
with_wpp_fields!(block_impl! { DmaWithPrePostBlockRow for DmaWithPrePostTraceRowPacked });
with_wpp_fields!(block_impl! { DmaWithPrePostBlockRow for CompactDmaTraceRow prefix wpp_ });
with_wpp_fields!(block_impl! { DmaWithPrePostBlockRow for CompactDmaTraceRowPacked prefix wpp_ });

with_loop_fields!(block_trait! {
    /// The columns of a `DmaLoop` row, as `DmaLoopSM` writes them. The fill writes every one of
    /// them on every row of its block -- a row of a sequence, or the padding -- so the buffer need
    /// not be zeroed.
    DmaLoopBlockRow
});
with_loop_fields!(block_impl! { DmaLoopBlockRow for DmaLoopTraceRow });
with_loop_fields!(block_impl! { DmaLoopBlockRow for DmaLoopTraceRowPacked });
with_loop_fields!(block_impl! { DmaLoopBlockRow for CompactDmaTraceRow prefix loop_ });
with_loop_fields!(block_impl! { DmaLoopBlockRow for CompactDmaTraceRowPacked prefix loop_ });

/// Writes the padding of the loop block into `row`: every column zero but @[seq_end], so the row
/// is not read as the continuation of a sequence (see @[padding] in `dma_loop.pil`).
#[inline(always)]
pub(crate) fn set_dma_loop_padding<F: PrimeField64, R: DmaLoopBlockRow<F>>(row: &mut R) {
    row.set_main_step(0);
    row.set_src64(0);
    row.set_dst64(0);
    row.set_count(0);
    row.set_seq_end(true);
    row.set_sel_memcpy(false);
    row.set_sel_memeq(false);
    row.set_sel_memset(false);
    row.set_sel_inputcpy(false);
    row.set_sel_memcpy_count_load(false);
    row.set_fill_byte(0);
    set_dma_loop_offset::<F, R>(row, 0);
    row.set_all_sel_op_from_1(&[false; DMA_LOOP_OPS_BY_ROW - 1]);
    row.set_all_read_bytes(&[0; 8 * DMA_LOOP_OPS_BY_ROW]);
}

/// Writes the one-hot `offset_1 .. offset_7` of a loop row. `offset` 0 leaves the seven at zero,
/// which is what the air reads as @[offset_0].
#[inline(always)]
pub(crate) fn set_dma_loop_offset<F: PrimeField64, R: DmaLoopBlockRow<F>>(row: &mut R, offset: u8) {
    debug_assert!(offset < 8);
    row.set_offset_1(offset == 1);
    row.set_offset_2(offset == 2);
    row.set_offset_3(offset == 3);
    row.set_offset_4(offset == 4);
    row.set_offset_5(offset == 5);
    row.set_offset_6(offset == 6);
    row.set_offset_7(offset == 7);
}

/// Declares a function returning every column of a block, in declaration order, as one comparable
/// value: the generated rows implement neither `PartialEq` nor a column iterator.
#[cfg(test)]
macro_rules! block_snapshot {
    ($name:ident, $trait:ident {
        scalars { $($s:ident: $st:ty),* $(,)? }
        arrays { $($a:ident: $at:ty),* $(,)? }
    }) => {
        paste::paste! {
            pub(crate) fn $name<F: PrimeField64, R: $trait<F>>(row: &R) -> Vec<u64> {
                let mut columns = vec![$( row.[<get_ $s>]() as u64 ),*];
                $( columns.extend(row.[<get_all_ $a>]().iter().map(|&v| v as u64)); )*
                columns
            }
        }
    };
}

#[cfg(test)]
with_wpp_fields!(block_snapshot! { wpp_snapshot, DmaWithPrePostBlockRow });
#[cfg(test)]
with_loop_fields!(block_snapshot! { loop_snapshot, DmaLoopBlockRow });

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_pil::DmaWithPrePostTraceRowOps;

    type Compact = CompactDmaTraceRow<Goldilocks>;

    /// Each block writes its own columns: filling one leaves the other where it was.
    #[test]
    fn the_blocks_of_a_fused_row_do_not_overlap() {
        let mut row = Compact::default();
        DmaWithPrePostBlockRow::<Goldilocks>::set_main_step(&mut row, 7);
        DmaWithPrePostBlockRow::<Goldilocks>::set_all_rb(&mut row, &[0xAB; 16]);
        DmaLoopBlockRow::<Goldilocks>::set_main_step(&mut row, 9);
        DmaLoopBlockRow::<Goldilocks>::set_all_read_bytes(&mut row, &[0xCD; 32]);

        assert_eq!(row.get_wpp_main_step(), 7);
        assert_eq!(row.get_loop_main_step(), 9);
        assert_eq!(row.get_all_wpp_rb(), [0xAB; 16]);
        assert_eq!(row.get_all_loop_read_bytes(), [0xCD; 32]);

        set_dma_loop_padding::<Goldilocks, _>(&mut row);
        assert_eq!(row.get_wpp_main_step(), 7, "the loop padding reached the wpp block");
        assert!(row.get_loop_seq_end());
        assert_eq!(row.get_loop_main_step(), 0);

        // Copying one block brings that block and nothing else.
        let mut padding = Compact::default();
        set_dma_loop_padding::<Goldilocks, _>(&mut padding);
        DmaLoopBlockRow::<Goldilocks>::set_main_step(&mut row, 9);
        DmaLoopBlockRow::<Goldilocks>::copy_block_from(&mut row, &padding);
        assert_eq!(loop_snapshot::<Goldilocks, _>(&row), loop_snapshot::<Goldilocks, _>(&padding));
        assert_eq!(row.get_wpp_main_step(), 7, "copying the loop block reached the wpp block");
        assert_eq!(row.get_all_wpp_rb(), [0xAB; 16]);
    }

    /// The standalone rows are reached through the same trait without a prefix.
    #[test]
    fn the_standalone_rows_are_their_own_block() {
        let mut row = DmaWithPrePostTraceRow::<Goldilocks>::default();
        DmaWithPrePostBlockRow::<Goldilocks>::set_l_count(&mut row, 300);
        assert_eq!(DmaWithPrePostTraceRowOps::<Goldilocks>::get_l_count(&row), 300);

        let mut row = DmaLoopTraceRow::<Goldilocks>::default();
        set_dma_loop_offset::<Goldilocks, _>(&mut row, 5);
        assert!(row.get_offset_5() && !row.get_offset_1() && !row.get_offset_7());
    }
}
