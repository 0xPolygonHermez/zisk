//! The fused `CompactMem` trace: three memory areas on the same rows, none of them stepping on
//! another.
//!
//! Two things have to hold for the fusion to be sound, and neither is visible from the fill of a
//! single area:
//!
//! 1. a block of a fused row must end up holding exactly what the standalone air would hold, and
//! 2. filling one block must leave the other two exactly as they were.
//!
//! (2) is not free. The `Mem` fill pads the tail of a segment by building one row and copying it
//! over every remaining row -- millions of them -- and on a fused row that copy must take the
//! `mem_` columns only. That is what [`MemLaneRow::copy_mem_block_from`] exists for, and what the
//! poison tests below pin.

use super::*;
use proofman_fields::Goldilocks;
use zisk_common::SegmentId;
use zisk_pil::{CompactMemTraceRow, MemTraceRow};
use zisk_sm_mem_common::{
    InputDataLaneRow, MemLaneRow, MemModuleSegmentCheckPoint, RomDataLaneRow, RAM_W_ADDR_INIT,
};

use crate::mem_sm::fill_mem_trace;
use crate::MemInput;

type Standalone = MemTraceRow<Goldilocks>;
type Fused = CompactMemTraceRow<Goldilocks>;

fn mem_lanes() -> usize {
    <Standalone as MemLaneRow<Goldilocks>>::mem_lanes_x_row()
}

/// A `Mem` segment of `n_addrs` addresses taking `slots_per_addr` slots each, and the operations
/// that fill it: one read then writes, the shape whose slot accounting is one slot per operation.
/// Same builder as `mem_sm::fill_tests`, kept local so the two test files stay independent.
fn segment_and_ops(
    n_addrs: u32,
    slots_per_addr: u32,
) -> (MemModuleSegmentCheckPoint, Vec<MemInput>) {
    let base = RAM_W_ADDR_INIT + 100;
    let mut seg = MemModuleSegmentCheckPoint::default();
    for a in 0..n_addrs {
        seg.add_addr_offset(base + a, a * slots_per_addr + 1);
    }

    let mut ops = Vec::new();
    let mut step = 1u64;
    for a in 0..n_addrs {
        for o in 0..slots_per_addr {
            ops.push(MemInput::new(
                base + a,
                o > 0,
                step,
                0xDEAD_0000_0000_0000 | ((a as u64) << 8) | o as u64,
            ));
            step += 1;
        }
    }
    (seg, ops)
}

fn rows_for(n_addrs: u32, slots_per_addr: u32) -> usize {
    ((n_addrs * slots_per_addr) as usize).div_ceil(mem_lanes()) + 1
}

/// Every `Mem` column of every lane, flattened, seen through the adapter so the standalone row and
/// the fused one can be compared. `previous_step` and `read_same_addr` are left out: they are the
/// air's `<==` columns, derived by the prover, and no fill writes them.
fn mem_block_snapshot<R: MemLaneRow<Goldilocks>>(rows: &[R]) -> Vec<u64> {
    let lanes = <R as MemLaneRow<Goldilocks>>::mem_lanes_x_row();
    let mut out = Vec::with_capacity(rows.len() * lanes * 11);
    for row in rows {
        for l in 0..lanes {
            out.push(MemLaneRow::<Goldilocks>::get_addr(row, l) as u64);
            out.push(MemLaneRow::<Goldilocks>::get_step(row, l));
            out.push(MemLaneRow::<Goldilocks>::get_sel(row, l) as u64);
            out.push(MemLaneRow::<Goldilocks>::get_addr_changes(row, l) as u64);
            out.push(MemLaneRow::<Goldilocks>::get_wr(row, l) as u64);
            out.push(MemLaneRow::<Goldilocks>::get_sel_dual(row, l) as u64);
            out.push(MemLaneRow::<Goldilocks>::get_step_dual(row, l));
            out.push(MemLaneRow::<Goldilocks>::get_value(row, l, 0) as u64);
            out.push(MemLaneRow::<Goldilocks>::get_value(row, l, 1) as u64);
            out.push(MemLaneRow::<Goldilocks>::get_l_increment(row, l) as u64);
            out.push(MemLaneRow::<Goldilocks>::get_h_increment(row, l) as u64);
        }
    }
    out
}

/// Writes recognizable values into the `input_` block, the way its own fill does.
fn stamp_input_block(rows: &mut [Fused], tag: u16) {
    let lanes = <Fused as InputDataLaneRow<Goldilocks>>::input_data_lanes_x_row();
    for (r, row) in rows.iter_mut().enumerate() {
        for l in 0..lanes {
            InputDataLaneRow::<Goldilocks>::set_addr(row, l, 0x4000_0000 + r as u32);
            InputDataLaneRow::<Goldilocks>::set_step(row, l, (r * lanes + l) as u64 + 1);
            InputDataLaneRow::<Goldilocks>::set_sel(row, l, l % 2 == 0);
            InputDataLaneRow::<Goldilocks>::set_addr_changes(row, l, l == 0);
            InputDataLaneRow::<Goldilocks>::set_is_free_read(row, l, l == 1);
            for i in 0..4 {
                InputDataLaneRow::<Goldilocks>::set_value_word(row, l, i, tag + i as u16);
            }
        }
    }
}

fn input_block_snapshot(rows: &[Fused]) -> Vec<u64> {
    let lanes = <Fused as InputDataLaneRow<Goldilocks>>::input_data_lanes_x_row();
    let mut out = Vec::new();
    for row in rows {
        for l in 0..lanes {
            out.push(InputDataLaneRow::<Goldilocks>::get_addr(row, l) as u64);
            out.push(InputDataLaneRow::<Goldilocks>::get_step(row, l));
            out.push(InputDataLaneRow::<Goldilocks>::get_sel(row, l) as u64);
            out.push(InputDataLaneRow::<Goldilocks>::get_addr_changes(row, l) as u64);
            out.push(InputDataLaneRow::<Goldilocks>::get_is_free_read(row, l) as u64);
            for i in 0..4 {
                out.push(InputDataLaneRow::<Goldilocks>::get_value_word(row, l, i) as u64);
            }
        }
    }
    out
}

/// Writes recognizable values into the `rom_` block, the way its own fill does.
fn stamp_rom_block(rows: &mut [Fused], tag: u32) {
    let lanes = <Fused as RomDataLaneRow<Goldilocks>>::rom_data_lanes_x_row();
    for (r, row) in rows.iter_mut().enumerate() {
        for l in 0..lanes {
            RomDataLaneRow::<Goldilocks>::set_addr(row, l, 0x8000_0000 + r as u32);
            RomDataLaneRow::<Goldilocks>::set_step(row, l, (r * lanes + l) as u64 + 3);
            RomDataLaneRow::<Goldilocks>::set_addr_change(row, l, l == 0);
            RomDataLaneRow::<Goldilocks>::set_value(row, l, 0, tag + r as u32);
            RomDataLaneRow::<Goldilocks>::set_value(row, l, 1, tag ^ l as u32);
        }
    }
}

fn rom_block_snapshot(rows: &[Fused]) -> Vec<u64> {
    let lanes = <Fused as RomDataLaneRow<Goldilocks>>::rom_data_lanes_x_row();
    let mut out = Vec::new();
    for row in rows {
        for l in 0..lanes {
            out.push(RomDataLaneRow::<Goldilocks>::get_addr(row, l) as u64);
            out.push(RomDataLaneRow::<Goldilocks>::get_step(row, l));
            out.push(RomDataLaneRow::<Goldilocks>::get_addr_change(row, l) as u64);
            out.push(RomDataLaneRow::<Goldilocks>::get_value(row, l, 0) as u64);
            out.push(RomDataLaneRow::<Goldilocks>::get_value(row, l, 1) as u64);
        }
    }
    out
}

fn run_mem_fill<R: MemLaneRow<Goldilocks>>(
    rows: &mut [R],
    seg: &MemModuleSegmentCheckPoint,
    ops: &[MemInput],
    n_ranges: usize,
) {
    let chunks =
        vec![ops.iter().map(|op| MemInput::new(op.addr, op.is_write, op.step, op.value)).collect()];
    let prev = MemPreviousSegment { addr: RAM_W_ADDR_INIT, step: 0, value: 0 };
    fill_mem_trace::<Goldilocks, R>(
        rows,
        MemOps::new(&chunks),
        seg,
        &prev,
        SegmentId(0),
        true,
        n_ranges,
    );
}

/// The `mem_` block of a fused row holds exactly what the `Mem` air's own row would hold -- lane
/// for lane, column for column, padding included.
#[test]
fn the_mem_block_of_a_fused_row_fills_like_its_own_air() {
    for n_ranges in [1, 3] {
        let (seg, ops) = segment_and_ops(7, 3);
        let n_rows = rows_for(7, 3);

        let mut standalone = vec![Standalone::default(); n_rows];
        run_mem_fill(&mut standalone, &seg, &ops, n_ranges);

        let mut fused = vec![Fused::default(); n_rows];
        run_mem_fill(&mut fused, &seg, &ops, n_ranges);

        assert_eq!(
            mem_block_snapshot(&standalone),
            mem_block_snapshot(&fused),
            "the fused mem block differs from the standalone one with {n_ranges} ranges"
        );
    }
}

/// Filling the `mem_` block leaves the other two blocks byte for byte as they were. The padding is
/// what makes this worth a test: it repeats one built row over the whole tail, and a whole-row copy
/// would take the `input_` and `rom_` columns with it.
#[test]
fn the_mem_fill_leaves_the_other_blocks_untouched() {
    // Few operations and many rows, so most of the trace is padding.
    let (seg, ops) = segment_and_ops(2, 2);
    let n_rows = rows_for(2, 2) + 8;

    let mut fused = vec![Fused::default(); n_rows];
    stamp_input_block(&mut fused, 0x1234);
    stamp_rom_block(&mut fused, 0x0BAD_0000);
    let input_before = input_block_snapshot(&fused);
    let rom_before = rom_block_snapshot(&fused);

    run_mem_fill(&mut fused, &seg, &ops, 1);

    assert_eq!(
        input_before,
        input_block_snapshot(&fused),
        "the mem fill wrote into the input_data block"
    );
    assert_eq!(
        rom_before,
        rom_block_snapshot(&fused),
        "the mem fill wrote into the rom_data block"
    );
}

/// And the other way round: writing the `input_` and `rom_` blocks after the `mem_` one leaves the
/// mem block alone, which is what lets the three fills run over the same rows in any order.
#[test]
fn writing_the_other_blocks_leaves_the_mem_block_untouched() {
    let (seg, ops) = segment_and_ops(5, 3);
    let n_rows = rows_for(5, 3) + 4;

    let mut fused = vec![Fused::default(); n_rows];
    run_mem_fill(&mut fused, &seg, &ops, 1);
    let mem_before = mem_block_snapshot(&fused);

    stamp_input_block(&mut fused, 0xFFFF);
    stamp_rom_block(&mut fused, 0xFFFF_FFFF);

    assert_eq!(
        mem_before,
        mem_block_snapshot(&fused),
        "writing the other blocks moved the mem block"
    );
}
