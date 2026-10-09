//! The fused `CompactBinary` row seen through the lane traits of the four binary fills.
//!
//! `CompactBinary` is the four binary airtemplates instanced inline, side by side, on one row (see
//! `state-machines/binary/pil/compact_binary.pil`): its generated row names the columns of each
//! block with that block's prefix -- `basic_b_op`, `add_a`, `add_hi_c_chunks`, `ext_op`, ... The
//! impls below are that renaming and nothing else: each lane trait is forwarded to the prefixed
//! setters, so every fill is written once, for its own air, and runs unchanged on the fused row.
//!
//! The one thing a fused row needs that a standalone one does not is `copy_block_from`: the padding
//! of a block is one row repeated over the tail of the instance, and on a fused row that copy must
//! take the block's columns only -- the other blocks belong to fills that have not run yet, or have
//! already run.

use crate::{
    lanes_x_row, BinaryAddHiLaneRow, BinaryAddLaneRow, BinaryBasicLaneRow, BinaryExtensionLaneRow,
    CHUNKS_X_ADD, CHUNKS_X_FULL_ADD, LIMBS_X_ADD,
};
use proofman_fields::PrimeField64;
use zisk_pil::{CompactBinaryTraceRow, CompactBinaryTraceRowPacked};

macro_rules! impl_compact_binary_rows {
    ($row:ident) => {
        impl<F: PrimeField64> BinaryBasicLaneRow<F> for $row<F> {
            const LANES_X_ROW: usize = lanes_x_row::COMPACT_BASIC;

            #[inline(always)]
            fn set_b_op(&mut self, lane: usize, value: u8) {
                self.set_basic_b_op(lane, value);
            }
            #[inline(always)]
            fn set_mode32(&mut self, lane: usize, value: bool) {
                self.set_basic_mode32(lane, value);
            }
            #[inline(always)]
            fn set_result_is_a(&mut self, lane: usize, value: bool) {
                self.set_basic_result_is_a(lane, value);
            }
            #[inline(always)]
            fn set_use_first_byte(&mut self, lane: usize, value: bool) {
                self.set_basic_use_first_byte(lane, value);
            }
            #[inline(always)]
            fn set_c_is_signed(&mut self, lane: usize, value: bool) {
                self.set_basic_c_is_signed(lane, value);
            }
            #[inline(always)]
            fn set_all_free_in_a(&mut self, lane: usize, values: &[u8; 8]) {
                for (j, &v) in values.iter().enumerate() {
                    self.set_basic_free_in_a(lane, j, v);
                }
            }
            #[inline(always)]
            fn set_all_free_in_b(&mut self, lane: usize, values: &[u8; 8]) {
                for (j, &v) in values.iter().enumerate() {
                    self.set_basic_free_in_b(lane, j, v);
                }
            }
            #[inline(always)]
            fn set_all_free_in_c(&mut self, lane: usize, values: &[u8; 8]) {
                for (j, &v) in values.iter().enumerate() {
                    self.set_basic_free_in_c(lane, j, v);
                }
            }
            #[inline(always)]
            fn set_all_carry(&mut self, lane: usize, values: &[u8; 8]) {
                for (j, &v) in values.iter().enumerate() {
                    self.set_basic_carry(lane, j, v);
                }
            }
            /// `basic_b_op_or_sext` and `basic_mode32_and_c_is_signed` are left out: they are the
            /// air's `<==` columns, derived by the prover, and the fill never writes them.
            #[inline(always)]
            fn copy_block_from(&mut self, src: &Self) {
                self.set_all_basic_b_op(&src.get_all_basic_b_op());
                self.set_all_basic_free_in_a(&src.get_all_basic_free_in_a());
                self.set_all_basic_free_in_b(&src.get_all_basic_free_in_b());
                self.set_all_basic_free_in_c(&src.get_all_basic_free_in_c());
                self.set_all_basic_carry(&src.get_all_basic_carry());
                self.set_all_basic_mode32(&src.get_all_basic_mode32());
                self.set_all_basic_result_is_a(&src.get_all_basic_result_is_a());
                self.set_all_basic_use_first_byte(&src.get_all_basic_use_first_byte());
                self.set_all_basic_c_is_signed(&src.get_all_basic_c_is_signed());
            }
        }

        impl<F: PrimeField64> BinaryAddLaneRow<F> for $row<F> {
            const LANES_X_ROW: usize = lanes_x_row::COMPACT_ADD;

            #[inline(always)]
            fn set_slot(
                &mut self,
                lane: usize,
                a: &[u32; LIMBS_X_ADD],
                b: &[u32; LIMBS_X_ADD],
                c_chunks: &[u16; CHUNKS_X_FULL_ADD],
                cout: &[bool; LIMBS_X_ADD],
                sh3add: bool,
            ) {
                for i in 0..LIMBS_X_ADD {
                    self.set_add_a(lane, i, a[i]);
                    self.set_add_b(lane, i, b[i]);
                    self.set_add_cout(lane, i, cout[i]);
                }
                for i in 0..CHUNKS_X_FULL_ADD {
                    self.set_add_c_chunks(lane, i, c_chunks[i]);
                }
                self.set_add_sel_sh3add(lane, sh3add);
            }
            #[inline(always)]
            fn copy_block_from(&mut self, src: &Self) {
                self.set_all_add_a(&src.get_all_add_a());
                self.set_all_add_b(&src.get_all_add_b());
                self.set_all_add_c_chunks(&src.get_all_add_c_chunks());
                self.set_all_add_cout(&src.get_all_add_cout());
                self.set_all_add_sel_sh3add(&src.get_all_add_sel_sh3add());
            }
        }

        impl<F: PrimeField64> BinaryAddHiLaneRow<F> for $row<F> {
            const LANES_X_ROW: usize = lanes_x_row::COMPACT_ADD_HI;

            #[inline(always)]
            fn set_slots(
                &mut self,
                a: &[u32],
                b: &[u32],
                c_chunks: &[[u16; CHUNKS_X_ADD]],
                sel: &[bool],
                sh3add: &[bool],
            ) {
                for lane in 0..lanes_x_row::COMPACT_ADD_HI {
                    self.set_add_hi_a(lane, a[lane]);
                    self.set_add_hi_b(lane, b[lane]);
                    for (i, &chunk) in c_chunks[lane].iter().enumerate() {
                        self.set_add_hi_c_chunks(lane, i, chunk);
                    }
                    self.set_add_hi_sel_b_hi_is_ff(lane, sel[lane]);
                    self.set_add_hi_sel_sh3add(lane, sh3add[lane]);
                }
            }
            #[inline(always)]
            fn copy_block_from(&mut self, src: &Self) {
                self.set_all_add_hi_a(&src.get_all_add_hi_a());
                self.set_all_add_hi_b(&src.get_all_add_hi_b());
                self.set_all_add_hi_c_chunks(&src.get_all_add_hi_c_chunks());
                self.set_all_add_hi_sel_b_hi_is_ff(&src.get_all_add_hi_sel_b_hi_is_ff());
                self.set_all_add_hi_sel_sh3add(&src.get_all_add_hi_sel_sh3add());
            }
        }

        impl<F: PrimeField64> BinaryExtensionLaneRow<F> for $row<F> {
            const LANES_X_ROW: usize = lanes_x_row::COMPACT_EXT;

            #[inline(always)]
            fn set_fields(
                &mut self,
                lane: usize,
                op: u8,
                free_in_a: &[u8; 8],
                free_in_b: u8,
                free_in_c: &[[u32; 2]; 8],
                op_is_shift: bool,
                op_is_combine: bool,
                free_in_b_bit6: bool,
                free_in_b_bit7: bool,
                op_is_chain: bool,
                op_is_chain_rev: bool,
                b: &[u32; 2],
            ) {
                self.set_ext_op(lane, op);
                for j in 0..8 {
                    self.set_ext_free_in_a(lane, j, free_in_a[j]);
                    self.set_ext_free_in_c(lane, j, 0, free_in_c[j][0]);
                    self.set_ext_free_in_c(lane, j, 1, free_in_c[j][1]);
                }
                self.set_ext_free_in_b(lane, free_in_b);
                self.set_ext_op_is_shift(lane, op_is_shift);
                self.set_ext_op_is_combine(lane, op_is_combine);
                self.set_ext_free_in_b_bit6(lane, free_in_b_bit6);
                self.set_ext_free_in_b_bit7(lane, free_in_b_bit7);
                self.set_ext_op_is_chain(lane, op_is_chain);
                self.set_ext_op_is_chain_rev(lane, op_is_chain_rev);
                self.set_ext_b(lane, 0, b[0]);
                self.set_ext_b(lane, 1, b[1]);
            }
            #[inline(always)]
            fn copy_block_from(&mut self, src: &Self) {
                self.set_all_ext_op(&src.get_all_ext_op());
                self.set_all_ext_free_in_a(&src.get_all_ext_free_in_a());
                self.set_all_ext_free_in_b(&src.get_all_ext_free_in_b());
                self.set_all_ext_free_in_c(&src.get_all_ext_free_in_c());
                self.set_all_ext_op_is_shift(&src.get_all_ext_op_is_shift());
                self.set_all_ext_op_is_combine(&src.get_all_ext_op_is_combine());
                self.set_all_ext_free_in_b_bit6(&src.get_all_ext_free_in_b_bit6());
                self.set_all_ext_free_in_b_bit7(&src.get_all_ext_free_in_b_bit7());
                self.set_all_ext_op_is_chain(&src.get_all_ext_op_is_chain());
                self.set_all_ext_op_is_chain_rev(&src.get_all_ext_op_is_chain_rev());
                self.set_all_ext_b(&src.get_all_ext_b());
            }
        }
    };
}

impl_compact_binary_rows!(CompactBinaryTraceRow);
impl_compact_binary_rows!(CompactBinaryTraceRowPacked);

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;

    type Row = CompactBinaryTraceRow<Goldilocks>;

    /// Copying one block of a fused row brings that block and nothing else: the padding of each
    /// block is written through this, over rows the other blocks may already hold.
    #[test]
    fn copying_a_block_leaves_the_other_blocks_alone() {
        let mut src = Row::default();
        BinaryBasicLaneRow::<Goldilocks>::set_b_op(&mut src, 0, 0x33);
        BinaryAddLaneRow::<Goldilocks>::set_slot(
            &mut src,
            0,
            &[7, 0],
            &[0, 0],
            &[7, 0, 0, 0],
            &[false, false],
            false,
        );
        BinaryExtensionLaneRow::<Goldilocks>::set_fields(
            &mut src,
            0,
            0x21,
            &[1; 8],
            3,
            &[[0; 2]; 8],
            true,
            false,
            false,
            false,
            false,
            false,
            &[0, 0],
        );

        let mut dst = Row::default();
        BinaryBasicLaneRow::<Goldilocks>::set_b_op(&mut dst, 0, 0x0A);
        BinaryExtensionLaneRow::<Goldilocks>::copy_block_from(&mut dst, &src);
        assert_eq!(dst.get_ext_op(0), 0x21, "the ext block was not copied");
        assert_eq!(dst.get_basic_b_op(0), 0x0A, "copying the ext block reached the basic block");
        assert_eq!(dst.get_add_a(0, 0), 0, "copying the ext block reached the add block");

        BinaryAddLaneRow::<Goldilocks>::copy_block_from(&mut dst, &src);
        assert_eq!(dst.get_add_a(0, 0), 7);
        assert_eq!(dst.get_basic_b_op(0), 0x0A, "copying the add block reached the basic block");

        BinaryBasicLaneRow::<Goldilocks>::copy_block_from(&mut dst, &src);
        assert_eq!(dst.get_basic_b_op(0), 0x33);
        assert_eq!(dst.get_ext_op(0), 0x21, "copying the basic block reached the ext block");
    }
}
