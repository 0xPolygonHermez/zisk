//! One fill, two traces: the row views the memory fills write through.
//!
//! Each memory area is proved by two airs -- its own (`Mem`, `InputData`, `RomData`) and the fused
//! `CompactMem`, which carries the three of them side by side on every row (see
//! `state-machines/mem/pil/compact_mem.pil`). The witness of an area is built the same way in both,
//! but the generated row types are different: `MemTraceRow` names its columns `addr`, `step`, ...
//! while `CompactMemTraceRow` names the same columns `mem_addr`, `mem_step`, ... alongside the
//! `input_*` and `rom_*` blocks.
//!
//! The traits below are that difference and nothing else: one method per column the fill writes,
//! implemented for the standalone row and for the fused one, so each fill is written once and
//! instantiated for whichever row it is filling. They are deliberately NOT derived from the
//! generated `*TraceRowOps` traits -- a blanket impl over those would collide with the `CompactMem`
//! impls, because coherence cannot see that `CompactMemTraceRow` does not implement them.
//!
//! The fills only ever touch their own block, which is what makes the fused trace work: the three
//! of them write into the same rows, each one leaving the other two blocks alone. The one place
//! that used to write a whole row -- the bulk padding of the `Mem` fill -- goes through
//! [`MemLaneRow::copy_mem_block_from`] instead.

use proofman_common::trace::TraceRow;
use proofman_fields::{Goldilocks, PrimeField64};
use zisk_pil::{
    CompactMemTraceRow, CompactMemTraceRowPacked, InputDataTraceRow, InputDataTraceRowOps,
    InputDataTraceRowPacked, MemTraceRow, MemTraceRowOps, MemTraceRowPacked, RomDataTraceRow,
    RomDataTraceRowOps, RomDataTraceRowPacked,
};

/// The columns of a `Mem` lane, as the `Mem` fill writes them.
pub trait MemLaneRow<F: PrimeField64>:
    TraceRow + Default + Copy + Send + Sync + std::fmt::Debug + 'static
{
    fn mem_lanes_x_row() -> usize;

    fn get_addr(&self, lane: usize) -> u32;
    fn set_addr(&mut self, lane: usize, value: u32);
    fn get_step(&self, lane: usize) -> u64;
    fn set_step(&mut self, lane: usize, value: u64);
    fn get_sel(&self, lane: usize) -> bool;
    fn set_sel(&mut self, lane: usize, value: bool);
    fn get_addr_changes(&self, lane: usize) -> bool;
    fn set_addr_changes(&mut self, lane: usize, value: bool);
    fn get_wr(&self, lane: usize) -> bool;
    fn set_wr(&mut self, lane: usize, value: bool);
    fn get_step_dual(&self, lane: usize) -> u64;
    fn set_step_dual(&mut self, lane: usize, value: u64);
    fn get_sel_dual(&self, lane: usize) -> bool;
    fn set_sel_dual(&mut self, lane: usize, value: bool);
    fn get_value(&self, lane: usize, index: usize) -> u32;
    fn set_value(&mut self, lane: usize, index: usize, value: u32);
    fn get_l_increment(&self, lane: usize) -> u32;
    fn set_l_increment(&mut self, lane: usize, value: u32);
    fn get_h_increment(&self, lane: usize) -> u16;
    fn set_h_increment(&mut self, lane: usize, value: u16);

    /// Overwrites the `Mem` block of this row with `src`'s, leaving every other block untouched.
    ///
    /// The padding of a segment repeats one row millions of times, so it is built once and copied.
    /// On a `Mem` row the block IS the row and this is a plain assignment; on a `CompactMem` row it
    /// must not take the `input_*` / `rom_*` columns with it -- those belong to fills that have not
    /// run yet, or have already run.
    fn copy_mem_block_from(&mut self, src: &Self);
}

/// The columns of an `InputData` lane, as the `InputData` fill writes them.
pub trait InputDataLaneRow<F: PrimeField64>:
    TraceRow + Default + Copy + Send + Sync + std::fmt::Debug + 'static
{
    fn input_data_lanes_x_row() -> usize;

    fn get_addr(&self, lane: usize) -> u32;
    fn set_addr(&mut self, lane: usize, value: u32);
    fn get_step(&self, lane: usize) -> u64;
    fn set_step(&mut self, lane: usize, value: u64);
    fn get_sel(&self, lane: usize) -> bool;
    fn set_sel(&mut self, lane: usize, value: bool);
    fn get_addr_changes(&self, lane: usize) -> bool;
    fn set_addr_changes(&mut self, lane: usize, value: bool);
    fn get_is_free_read(&self, lane: usize) -> bool;
    fn set_is_free_read(&mut self, lane: usize, value: bool);
    fn get_value_word(&self, lane: usize, index: usize) -> u16;
    fn set_value_word(&mut self, lane: usize, index: usize, value: u16);

    /// Overwrites the `InputData` block of this row with `src`'s, leaving every other block
    /// untouched. Same role as [`MemLaneRow::copy_mem_block_from`]: the padding of a segment is
    /// one row repeated, built once and copied in parallel.
    fn copy_input_block_from(&mut self, src: &Self);
}

/// The columns of a `RomData` lane, as the `RomData` fill writes them.
pub trait RomDataLaneRow<F: PrimeField64>:
    TraceRow + Default + Copy + Send + Sync + std::fmt::Debug + 'static
{
    fn rom_data_lanes_x_row() -> usize;

    fn get_addr(&self, lane: usize) -> u32;
    fn set_addr(&mut self, lane: usize, value: u32);
    fn get_step(&self, lane: usize) -> u64;
    fn set_step(&mut self, lane: usize, value: u64);
    fn get_addr_change(&self, lane: usize) -> bool;
    fn set_addr_change(&mut self, lane: usize, value: bool);
    fn get_value(&self, lane: usize, index: usize) -> u32;
    fn set_value(&mut self, lane: usize, index: usize, value: u32);

    /// Overwrites the `RomData` block of this row with `src`'s, leaving every other block
    /// untouched. See [`InputDataLaneRow::copy_input_block_from`].
    fn copy_rom_block_from(&mut self, src: &Self);
}

/// `MemLaneRow` for a row that is nothing but a `Mem` block: every accessor is the generated one,
/// and the block copy is the row copy.
macro_rules! impl_mem_lane_row_standalone {
    ($row:ident) => {
        impl<F: PrimeField64> MemLaneRow<F> for $row<F> {
            #[inline(always)]
            fn mem_lanes_x_row() -> usize {
                <$row<Goldilocks> as Default>::default().get_all_addr().len()
            }
            impl_mem_lane_row_standalone!(@fwd get_addr, set_addr, u32);
            impl_mem_lane_row_standalone!(@fwd get_step, set_step, u64);
            impl_mem_lane_row_standalone!(@fwd get_sel, set_sel, bool);
            impl_mem_lane_row_standalone!(@fwd get_addr_changes, set_addr_changes, bool);
            impl_mem_lane_row_standalone!(@fwd get_wr, set_wr, bool);
            impl_mem_lane_row_standalone!(@fwd get_step_dual, set_step_dual, u64);
            impl_mem_lane_row_standalone!(@fwd get_sel_dual, set_sel_dual, bool);
            impl_mem_lane_row_standalone!(@fwd get_l_increment, set_l_increment, u32);
            impl_mem_lane_row_standalone!(@fwd get_h_increment, set_h_increment, u16);

            #[inline(always)]
            fn get_value(&self, lane: usize, index: usize) -> u32 {
                MemTraceRowOps::get_value(self, lane, index)
            }
            #[inline(always)]
            fn set_value(&mut self, lane: usize, index: usize, value: u32) {
                MemTraceRowOps::set_value(self, lane, index, value)
            }
            #[inline(always)]
            fn copy_mem_block_from(&mut self, src: &Self) {
                *self = *src;
            }
        }
    };
    (@fwd $get:ident, $set:ident, $ty:ty) => {
        #[inline(always)]
        fn $get(&self, lane: usize) -> $ty {
            MemTraceRowOps::$get(self, lane)
        }
        #[inline(always)]
        fn $set(&mut self, lane: usize, value: $ty) {
            MemTraceRowOps::$set(self, lane, value)
        }
    };
}

impl_mem_lane_row_standalone!(MemTraceRow);
impl_mem_lane_row_standalone!(MemTraceRowPacked);

/// `MemLaneRow` for the fused row: the same columns under their `mem_` names, and a block copy
/// that touches those columns only.
macro_rules! impl_mem_lane_row_compact {
    ($row:ident) => {
        impl<F: PrimeField64> MemLaneRow<F> for $row<F> {
            #[inline(always)]
            fn mem_lanes_x_row() -> usize {
                <$row<Goldilocks> as Default>::default().get_all_mem_addr().len()
            }
            impl_mem_lane_row_compact!(@fwd $row, get_addr, set_addr, get_mem_addr, set_mem_addr, u32);
            impl_mem_lane_row_compact!(@fwd $row, get_step, set_step, get_mem_step, set_mem_step, u64);
            impl_mem_lane_row_compact!(@fwd $row, get_sel, set_sel, get_mem_sel, set_mem_sel, bool);
            impl_mem_lane_row_compact!(@fwd $row, get_addr_changes, set_addr_changes, get_mem_addr_changes, set_mem_addr_changes, bool);
            impl_mem_lane_row_compact!(@fwd $row, get_wr, set_wr, get_mem_wr, set_mem_wr, bool);
            impl_mem_lane_row_compact!(@fwd $row, get_step_dual, set_step_dual, get_mem_step_dual, set_mem_step_dual, u64);
            impl_mem_lane_row_compact!(@fwd $row, get_sel_dual, set_sel_dual, get_mem_sel_dual, set_mem_sel_dual, bool);
            impl_mem_lane_row_compact!(@fwd $row, get_l_increment, set_l_increment, get_mem_l_increment, set_mem_l_increment, u32);
            impl_mem_lane_row_compact!(@fwd $row, get_h_increment, set_h_increment, get_mem_h_increment, set_mem_h_increment, u16);

            #[inline(always)]
            fn get_value(&self, lane: usize, index: usize) -> u32 {
                $row::get_mem_value(self, lane, index)
            }
            #[inline(always)]
            fn set_value(&mut self, lane: usize, index: usize, value: u32) {
                $row::set_mem_value(self, lane, index, value)
            }
            /// `mem_previous_step` and `mem_read_same_addr` are left out: they are the air's
            /// `<==` columns, derived by the prover, and the fill never writes them either.
            #[inline(always)]
            fn copy_mem_block_from(&mut self, src: &Self) {
                self.set_all_mem_addr(&src.get_all_mem_addr());
                self.set_all_mem_step(&src.get_all_mem_step());
                self.set_all_mem_sel(&src.get_all_mem_sel());
                self.set_all_mem_addr_changes(&src.get_all_mem_addr_changes());
                self.set_all_mem_value(&src.get_all_mem_value());
                self.set_all_mem_wr(&src.get_all_mem_wr());
                self.set_all_mem_step_dual(&src.get_all_mem_step_dual());
                self.set_all_mem_sel_dual(&src.get_all_mem_sel_dual());
                self.set_all_mem_l_increment(&src.get_all_mem_l_increment());
                self.set_all_mem_h_increment(&src.get_all_mem_h_increment());
            }
        }
    };
    (@fwd $row:ident, $get:ident, $set:ident, $src_get:ident, $src_set:ident, $ty:ty) => {
        #[inline(always)]
        fn $get(&self, lane: usize) -> $ty {
            $row::$src_get(self, lane)
        }
        #[inline(always)]
        fn $set(&mut self, lane: usize, value: $ty) {
            $row::$src_set(self, lane, value)
        }
    };
}

impl_mem_lane_row_compact!(CompactMemTraceRow);
impl_mem_lane_row_compact!(CompactMemTraceRowPacked);

macro_rules! impl_input_data_lane_row_standalone {
    ($row:ident) => {
        impl<F: PrimeField64> InputDataLaneRow<F> for $row<F> {
            #[inline(always)]
            fn input_data_lanes_x_row() -> usize {
                <$row<Goldilocks> as Default>::default().get_all_addr().len()
            }
            impl_input_data_lane_row_standalone!(@fwd get_addr, set_addr, u32);
            impl_input_data_lane_row_standalone!(@fwd get_step, set_step, u64);
            impl_input_data_lane_row_standalone!(@fwd get_sel, set_sel, bool);
            impl_input_data_lane_row_standalone!(@fwd get_addr_changes, set_addr_changes, bool);
            impl_input_data_lane_row_standalone!(@fwd get_is_free_read, set_is_free_read, bool);

            #[inline(always)]
            fn get_value_word(&self, lane: usize, index: usize) -> u16 {
                InputDataTraceRowOps::get_value_word(self, lane, index)
            }
            #[inline(always)]
            fn set_value_word(&mut self, lane: usize, index: usize, value: u16) {
                InputDataTraceRowOps::set_value_word(self, lane, index, value)
            }
            #[inline(always)]
            fn copy_input_block_from(&mut self, src: &Self) {
                *self = *src;
            }
        }
    };
    (@fwd $get:ident, $set:ident, $ty:ty) => {
        #[inline(always)]
        fn $get(&self, lane: usize) -> $ty {
            InputDataTraceRowOps::$get(self, lane)
        }
        #[inline(always)]
        fn $set(&mut self, lane: usize, value: $ty) {
            InputDataTraceRowOps::$set(self, lane, value)
        }
    };
}

impl_input_data_lane_row_standalone!(InputDataTraceRow);
impl_input_data_lane_row_standalone!(InputDataTraceRowPacked);

macro_rules! impl_input_data_lane_row_compact {
    ($row:ident) => {
        impl<F: PrimeField64> InputDataLaneRow<F> for $row<F> {
            #[inline(always)]
            fn input_data_lanes_x_row() -> usize {
                <$row<Goldilocks> as Default>::default().get_all_input_addr().len()
            }
            impl_input_data_lane_row_compact!(@fwd $row, get_addr, set_addr, get_input_addr, set_input_addr, u32);
            impl_input_data_lane_row_compact!(@fwd $row, get_step, set_step, get_input_step, set_input_step, u64);
            impl_input_data_lane_row_compact!(@fwd $row, get_sel, set_sel, get_input_sel, set_input_sel, bool);
            impl_input_data_lane_row_compact!(@fwd $row, get_addr_changes, set_addr_changes, get_input_addr_changes, set_input_addr_changes, bool);
            impl_input_data_lane_row_compact!(@fwd $row, get_is_free_read, set_is_free_read, get_input_is_free_read, set_input_is_free_read, bool);

            #[inline(always)]
            fn get_value_word(&self, lane: usize, index: usize) -> u16 {
                $row::get_input_value_word(self, lane, index)
            }
            #[inline(always)]
            fn set_value_word(&mut self, lane: usize, index: usize, value: u16) {
                $row::set_input_value_word(self, lane, index, value)
            }
            #[inline(always)]
            fn copy_input_block_from(&mut self, src: &Self) {
                self.set_all_input_addr(&src.get_all_input_addr());
                self.set_all_input_step(&src.get_all_input_step());
                self.set_all_input_sel(&src.get_all_input_sel());
                self.set_all_input_addr_changes(&src.get_all_input_addr_changes());
                self.set_all_input_value_word(&src.get_all_input_value_word());
                self.set_all_input_is_free_read(&src.get_all_input_is_free_read());
            }
        }
    };
    (@fwd $row:ident, $get:ident, $set:ident, $src_get:ident, $src_set:ident, $ty:ty) => {
        #[inline(always)]
        fn $get(&self, lane: usize) -> $ty {
            $row::$src_get(self, lane)
        }
        #[inline(always)]
        fn $set(&mut self, lane: usize, value: $ty) {
            $row::$src_set(self, lane, value)
        }
    };
}

impl_input_data_lane_row_compact!(CompactMemTraceRow);
impl_input_data_lane_row_compact!(CompactMemTraceRowPacked);

macro_rules! impl_rom_data_lane_row_standalone {
    ($row:ident) => {
        impl<F: PrimeField64> RomDataLaneRow<F> for $row<F> {
            #[inline(always)]
            fn rom_data_lanes_x_row() -> usize {
                <$row<Goldilocks> as Default>::default().get_all_addr().len()
            }
            impl_rom_data_lane_row_standalone!(@fwd get_addr, set_addr, u32);
            impl_rom_data_lane_row_standalone!(@fwd get_step, set_step, u64);
            impl_rom_data_lane_row_standalone!(@fwd get_addr_change, set_addr_change, bool);

            #[inline(always)]
            fn get_value(&self, lane: usize, index: usize) -> u32 {
                RomDataTraceRowOps::get_value(self, lane, index)
            }
            #[inline(always)]
            fn set_value(&mut self, lane: usize, index: usize, value: u32) {
                RomDataTraceRowOps::set_value(self, lane, index, value)
            }
            #[inline(always)]
            fn copy_rom_block_from(&mut self, src: &Self) {
                *self = *src;
            }
        }
    };
    (@fwd $get:ident, $set:ident, $ty:ty) => {
        #[inline(always)]
        fn $get(&self, lane: usize) -> $ty {
            RomDataTraceRowOps::$get(self, lane)
        }
        #[inline(always)]
        fn $set(&mut self, lane: usize, value: $ty) {
            RomDataTraceRowOps::$set(self, lane, value)
        }
    };
}

impl_rom_data_lane_row_standalone!(RomDataTraceRow);
impl_rom_data_lane_row_standalone!(RomDataTraceRowPacked);

macro_rules! impl_rom_data_lane_row_compact {
    ($row:ident) => {
        impl<F: PrimeField64> RomDataLaneRow<F> for $row<F> {
            #[inline(always)]
            fn rom_data_lanes_x_row() -> usize {
                <$row<Goldilocks> as Default>::default().get_all_rom_addr().len()
            }
            impl_rom_data_lane_row_compact!(@fwd $row, get_addr, set_addr, get_rom_addr, set_rom_addr, u32);
            impl_rom_data_lane_row_compact!(@fwd $row, get_step, set_step, get_rom_step, set_rom_step, u64);
            impl_rom_data_lane_row_compact!(@fwd $row, get_addr_change, set_addr_change, get_rom_addr_change, set_rom_addr_change, bool);

            #[inline(always)]
            fn get_value(&self, lane: usize, index: usize) -> u32 {
                $row::get_rom_value(self, lane, index)
            }
            #[inline(always)]
            fn set_value(&mut self, lane: usize, index: usize, value: u32) {
                $row::set_rom_value(self, lane, index, value)
            }
            #[inline(always)]
            fn copy_rom_block_from(&mut self, src: &Self) {
                self.set_all_rom_addr_change(&src.get_all_rom_addr_change());
                self.set_all_rom_addr(&src.get_all_rom_addr());
                self.set_all_rom_step(&src.get_all_rom_step());
                self.set_all_rom_value(&src.get_all_rom_value());
            }
        }
    };
    (@fwd $row:ident, $get:ident, $set:ident, $src_get:ident, $src_set:ident, $ty:ty) => {
        #[inline(always)]
        fn $get(&self, lane: usize) -> $ty {
            $row::$src_get(self, lane)
        }
        #[inline(always)]
        fn $set(&mut self, lane: usize, value: $ty) {
            $row::$src_set(self, lane, value)
        }
    };
}

impl_rom_data_lane_row_compact!(CompactMemTraceRow);
impl_rom_data_lane_row_compact!(CompactMemTraceRowPacked);

#[cfg(test)]
mod tests {
    use super::*;

    /// The fused row must carry each area's lanes in the same layout as the air it replaces --
    /// same lanes per row -- or the first segment planned for that air would not fit it.
    #[test]
    fn compact_row_keeps_the_lane_counts_of_the_airs_it_fuses() {
        assert_eq!(
            <CompactMemTraceRow<Goldilocks> as MemLaneRow<Goldilocks>>::mem_lanes_x_row(),
            <MemTraceRow<Goldilocks> as MemLaneRow<Goldilocks>>::mem_lanes_x_row()
        );
        assert_eq!(
            <CompactMemTraceRow<Goldilocks> as InputDataLaneRow<Goldilocks>>::input_data_lanes_x_row(),
            <InputDataTraceRow<Goldilocks> as InputDataLaneRow<Goldilocks>>::input_data_lanes_x_row()
        );
        assert_eq!(
            <CompactMemTraceRow<Goldilocks> as RomDataLaneRow<Goldilocks>>::rom_data_lanes_x_row(),
            <RomDataTraceRow<Goldilocks> as RomDataLaneRow<Goldilocks>>::rom_data_lanes_x_row()
        );
    }

    /// Copying the `Mem` block of a fused row must leave the other two blocks where they were.
    #[test]
    fn copy_mem_block_leaves_the_other_blocks_alone() {
        let mut src = CompactMemTraceRow::<Goldilocks>::default();
        MemLaneRow::<Goldilocks>::set_addr(&mut src, 0, 0x1234);
        MemLaneRow::<Goldilocks>::set_sel(&mut src, 0, true);
        InputDataLaneRow::<Goldilocks>::set_addr(&mut src, 0, 0xAAAA);
        RomDataLaneRow::<Goldilocks>::set_addr(&mut src, 0, 0xBBBB);

        let mut dst = CompactMemTraceRow::<Goldilocks>::default();
        InputDataLaneRow::<Goldilocks>::set_addr(&mut dst, 0, 0x1111);
        RomDataLaneRow::<Goldilocks>::set_addr(&mut dst, 0, 0x2222);

        MemLaneRow::<Goldilocks>::copy_mem_block_from(&mut dst, &src);

        assert_eq!(MemLaneRow::<Goldilocks>::get_addr(&dst, 0), 0x1234);
        assert!(MemLaneRow::<Goldilocks>::get_sel(&dst, 0));
        assert_eq!(InputDataLaneRow::<Goldilocks>::get_addr(&dst, 0), 0x1111);
        assert_eq!(RomDataLaneRow::<Goldilocks>::get_addr(&dst, 0), 0x2222);
    }

    /// Same for the other two blocks: each copy takes its own block and nothing else.
    #[test]
    fn copy_input_and_rom_blocks_leave_the_other_blocks_alone() {
        let mut src = CompactMemTraceRow::<Goldilocks>::default();
        MemLaneRow::<Goldilocks>::set_addr(&mut src, 0, 0x1234);
        InputDataLaneRow::<Goldilocks>::set_addr(&mut src, 0, 0xAAAA);
        InputDataLaneRow::<Goldilocks>::set_value_word(&mut src, 0, 3, 0x5555);
        InputDataLaneRow::<Goldilocks>::set_is_free_read(&mut src, 0, true);
        RomDataLaneRow::<Goldilocks>::set_addr(&mut src, 0, 0xBBBB);
        RomDataLaneRow::<Goldilocks>::set_value(&mut src, 0, 1, 0x7777);
        RomDataLaneRow::<Goldilocks>::set_addr_change(&mut src, 0, true);

        let mut dst = CompactMemTraceRow::<Goldilocks>::default();
        MemLaneRow::<Goldilocks>::set_addr(&mut dst, 0, 0x0001);
        RomDataLaneRow::<Goldilocks>::set_addr(&mut dst, 0, 0x2222);

        InputDataLaneRow::<Goldilocks>::copy_input_block_from(&mut dst, &src);
        assert_eq!(InputDataLaneRow::<Goldilocks>::get_addr(&dst, 0), 0xAAAA);
        assert_eq!(InputDataLaneRow::<Goldilocks>::get_value_word(&dst, 0, 3), 0x5555);
        assert!(InputDataLaneRow::<Goldilocks>::get_is_free_read(&dst, 0));
        assert_eq!(MemLaneRow::<Goldilocks>::get_addr(&dst, 0), 0x0001);
        assert_eq!(RomDataLaneRow::<Goldilocks>::get_addr(&dst, 0), 0x2222);

        RomDataLaneRow::<Goldilocks>::copy_rom_block_from(&mut dst, &src);
        assert_eq!(RomDataLaneRow::<Goldilocks>::get_addr(&dst, 0), 0xBBBB);
        assert_eq!(RomDataLaneRow::<Goldilocks>::get_value(&dst, 0, 1), 0x7777);
        assert!(RomDataLaneRow::<Goldilocks>::get_addr_change(&dst, 0));
        assert_eq!(MemLaneRow::<Goldilocks>::get_addr(&dst, 0), 0x0001);
        assert_eq!(InputDataLaneRow::<Goldilocks>::get_addr(&dst, 0), 0xAAAA);
    }
}
