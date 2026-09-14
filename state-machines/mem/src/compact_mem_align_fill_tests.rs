//! The fused `CompactMemAlign` trace: the two mem-align airs on the same rows, neither stepping on
//! the other.
//!
//! Two things have to hold for the fusion to be sound, and neither is visible from the fill of a
//! single air:
//!
//! 1. a block of a fused row must end up holding exactly what the standalone air would hold, laid
//!    out over the lanes the virtual rows map to, and
//! 2. filling one block must leave the other exactly as it was.
//!
//! (2) is not free. Both fills write EVERY column of their block, padding included -- the trace
//! buffer comes from the recycled pool and is not zeroed -- so the writers are one wrong column
//! name away from wiping the other block. That is what the poison tests below pin.
//!
//! Every test runs against the two airs of the family, which are instantiated with different lane
//! counts: the small one packs one `MemAlign` row and two byte rows per row, the tall one two and
//! four. So the same tests cover a block whose lanes chain inside a row and one whose sub-programs
//! roll over into the next row.
//!
//! What the sub-programs put in a row is not re-tested here: it is the code the standalone air is
//! filled with, run unchanged (`prove_mem_align_op`), and the tests stamp recognisable rows in its
//! place. What IS tested is where those rows land.

use super::*;
use proofman_fields::Goldilocks;

use crate::mem_align_byte_sm::fill_bytes_block_rows;
use crate::ByteRowValues;

/// An unaligned operation whose sub-program takes `rows` rows: the shapes the air proves.
fn op_of(rows: usize, step: u64) -> MemAlignInput {
    // offset 1 keeps the access unaligned; the width and the direction pick the sub-program.
    let (width, is_write) = match rows {
        2 => (4, false), // RV:    one word, read
        3 => (4, true),  // RWV:   one word, write
        5 => (8, true),  // RWVWR: two words, write
        _ => panic!("no sub-program takes {rows} rows"),
    };
    MemAlignInput {
        addr: 0xA000_0001 + step as u32 * 8,
        is_write,
        width,
        step,
        value: 0xDEAD_0000 | step,
        mem_values: [0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210],
    }
}

/// A one-byte operation, which is what the `bytes_` block proves.
fn byte_op_of(step: u64, is_write: bool) -> MemAlignInput {
    MemAlignInput {
        addr: 0xA000_0000 + step as u32 * 8 + (step as u32 % 8),
        is_write,
        width: 1,
        step,
        value: step & 0xFF,
        mem_values: [0x0123_4567_89AB_CDEF, 0],
    }
}

/// Stamps each row of a sub-program with what identifies it -- the operation's step and the row's
/// place in it -- instead of the real witness. Where the rows land is what the fill decides.
fn stamp(input: &MemAlignInput, scratch: &mut [MemAlignTraceRow<Goldilocks>]) {
    for (index, row) in scratch.iter_mut().enumerate() {
        *row = Default::default();
        row.set_step(input.step);
        row.set_pc(index as u8);
        row.set_addr(input.addr);
        row.set_all_reg(&[1, 2, 3, 4, 5, 6, 7, 8]);
    }
}

/// The cuts a parallel fill is split at: whatever the lane count and the sub-program sizes, they
/// must cover every operation and land on a physical row boundary, or two threads would write the
/// same row.
#[test]
fn the_groups_of_a_parallel_fill_never_share_a_row() {
    // Sub-program sizes that do not divide any lane count evenly, so the cuts have to hunt for a
    // boundary rather than fall on one.
    let sizes = [2usize, 5, 3, 2, 2, 5, 5, 3, 2, 3, 5, 2, 3, 2, 5];

    let mut starts = vec![0usize];
    for size in sizes {
        starts.push(starts.last().unwrap() + size);
    }

    for lanes in [1usize, 2, 4, 8] {
        for n_ranges in 1..=8 {
            let cuts = group_cuts(&starts, lanes, n_ranges);

            assert_eq!(
                cuts[0], 0,
                "lanes {lanes}, {n_ranges} ranges: the first group must begin at the first operation"
            );
            assert_eq!(
                *cuts.last().unwrap(),
                sizes.len(),
                "lanes {lanes}, {n_ranges} ranges: the last group must end at the last operation"
            );
            for pair in cuts.windows(2) {
                assert!(
                    pair[0] < pair[1],
                    "lanes {lanes}, {n_ranges} ranges: an empty or backwards group {pair:?}"
                );
            }
            for &cut in &cuts[1..cuts.len() - 1] {
                assert_eq!(
                    starts[cut] % lanes,
                    0,
                    "lanes {lanes}, {n_ranges} ranges: the group ending at operation {cut} ends \
                     inside a row"
                );
            }
        }
    }
}

/// The split between the two blocks has to be the very one the collector counted, and the one case
/// where "one byte wide" is not a byte operation is a write whose value does not fit in a byte.
#[test]
fn a_byte_operation_is_the_one_the_collector_counts_as_one() {
    let mut read = byte_op_of(1, false);
    assert!(is_byte_align_op(&read), "a one-byte read is a byte operation");

    read.width = 4;
    assert!(!is_byte_align_op(&read), "a four-byte read is not a byte operation");

    let mut write = byte_op_of(2, true);
    write.value = 0xFF;
    assert!(is_byte_align_op(&write), "a one-byte write of a byte is a byte operation");

    write.value = 0x1FF;
    assert!(
        !is_byte_align_op(&write),
        "a one-byte write whose value does not fit in a byte is proved by the full block"
    );
}

/// The fill tests, emitted once per air of the family: they only differ in the row type, and with it
/// in how many lanes of each block a row carries.
macro_rules! fill_tests_for {
    ($air:ident, $row:ident) => {
        mod $air {
            use super::*;
            use zisk_pil::$row;

            type Fused = $row<Goldilocks>;

            fn full_lanes() -> usize {
                <Fused as MemAlignFullBlockRow<Goldilocks>>::full_lanes_x_row()
            }

            fn bytes_lanes() -> usize {
                <Fused as MemAlignBytesBlockRow<Goldilocks>>::bytes_lanes_x_row()
            }

            /// Every `full_*` column of every lane, in a fixed order.
            fn full_block_snapshot(rows: &[Fused]) -> Vec<u64> {
                let mut out = Vec::new();
                for row in rows {
                    for lane in 0..full_lanes() {
                        out.push(row.get_full_addr(lane) as u64);
                        out.push(row.get_full_offset(lane) as u64);
                        out.push(row.get_full_width(lane) as u64);
                        out.push(row.get_full_wr(lane) as u64);
                        out.push(row.get_full_pc(lane) as u64);
                        out.push(row.get_full_reset(lane) as u64);
                        out.push(row.get_full_sel_up_to_down(lane) as u64);
                        out.push(row.get_full_sel_down_to_up(lane) as u64);
                        out.push(row.get_full_is_non_aligned_op(lane) as u64);
                        out.push(row.get_full_sel_w_lt8(lane) as u64);
                        out.push(row.get_full_sel_w_lt4(lane) as u64);
                        out.push(row.get_full_sel_w_lt2(lane) as u64);
                        out.push(row.get_full_step(lane));
                        for i in 0..8 {
                            out.push(row.get_full_reg(lane, i) as u64);
                            out.push(row.get_full_sel(lane, i) as u64);
                        }
                        for i in 0..2 {
                            out.push(row.get_full_value(lane, i) as u64);
                        }
                    }
                }
                out
            }

            /// Every `bytes_*` column of every lane, in a fixed order.
            fn bytes_block_snapshot(rows: &[Fused]) -> Vec<u64> {
                let mut out = Vec::new();
                for row in rows {
                    for lane in 0..bytes_lanes() {
                        out.push(row.get_bytes_sel_high_4b(lane) as u64);
                        out.push(row.get_bytes_sel_high_2b(lane) as u64);
                        out.push(row.get_bytes_sel_high_b(lane) as u64);
                        out.push(row.get_bytes_direct_value(lane) as u64);
                        out.push(row.get_bytes_composed_value(lane) as u64);
                        out.push(row.get_bytes_value_16b(lane) as u64);
                        out.push(row.get_bytes_value_8b(lane) as u64);
                        out.push(row.get_bytes_byte_value(lane) as u64);
                        out.push(row.get_bytes_addr_w(lane) as u64);
                        out.push(row.get_bytes_step(lane));
                        out.push(row.get_bytes_is_write(lane) as u64);
                        out.push(row.get_bytes_written_composed_value(lane) as u64);
                        out.push(row.get_bytes_written_byte_value(lane) as u64);
                        out.push(row.get_bytes_mem_write_values(lane, 0) as u64);
                        out.push(row.get_bytes_mem_write_values(lane, 1) as u64);
                        out.push(row.get_bytes_bus_byte(lane) as u64);
                    }
                }
                out
            }

            /// Writes recognisable values into the `full_` block, the way its own fill does.
            fn poison_full_block(rows: &mut [Fused]) {
                for (index, row) in rows.iter_mut().enumerate() {
                    for lane in 0..full_lanes() {
                        let tag = (index * full_lanes() + lane) as u64;
                        let mut src: MemAlignTraceRow<Goldilocks> = Default::default();
                        src.set_addr(0xBADC_0DE0 + tag as u32);
                        src.set_step(tag + 1);
                        src.set_pc(tag as u8);
                        src.set_reset(true);
                        src.set_all_reg(&[9, 8, 7, 6, 5, 4, 3, 2]);
                        src.set_all_sel(&[true, false, true, false, true, false, true, false]);
                        src.set_all_value(&[0xFFFF_0000, 0x0000_FFFF]);
                        row.set_full_lane(lane, &src);
                    }
                }
            }

            /// Writes recognisable values into the `bytes_` block, the way its own fill does.
            fn poison_bytes_block(rows: &mut [Fused]) {
                for (index, row) in rows.iter_mut().enumerate() {
                    for lane in 0..bytes_lanes() {
                        let tag = (index * bytes_lanes() + lane) as u32;
                        let values = ByteRowValues {
                            sel_high_4b: true,
                            sel_high_2b: false,
                            sel_high_b: true,
                            direct_value: 0xDEAD_0000 + tag,
                            composed_value: 0xBEEF_0000 + tag,
                            value_16b: tag as u16,
                            value_8b: tag as u8,
                            byte_value: !(tag as u8),
                            addr_w: 0x0BAD_0000 + tag,
                            step: tag as u64 + 7,
                            is_write: true,
                            written_composed_value: 0xC0DE_0000 + tag,
                            written_byte_value: tag as u8,
                            mem_write_values: [0x1111_0000 + tag, 0x2222_0000 + tag],
                        };
                        row.set_bytes_lane(lane, &values);
                    }
                }
            }

            /// The sub-programs land one after another over the virtual rows -- across the lanes of
            /// a row and on into the next one -- and what follows them is padding.
            #[test]
            fn the_full_block_lands_one_sub_program_after_another() {
                let ops: Vec<MemAlignInput> = [2usize, 5, 3, 2]
                    .iter()
                    .enumerate()
                    .map(|(i, &r)| op_of(r, i as u64 + 1))
                    .collect();
                let ops: Vec<&MemAlignInput> = ops.iter().collect();
                let slots: usize = ops.iter().map(|op| MemAlignSM::<Goldilocks>::op_rows(op)).sum();
                assert_eq!(slots, 12);

                // Three virtual rows more than the operations need, so the padding has a tail to
                // write.
                let mut rows = vec![Fused::default(); (slots + 3).div_ceil(full_lanes())];
                let (used, _) = fill_full_block_rows(&ops, &mut rows, &stamp);
                assert_eq!(used, slots);

                let mut slot = 0;
                for op in &ops {
                    for index in 0..MemAlignSM::<Goldilocks>::op_rows(op) {
                        let (row, lane) = (slot / full_lanes(), slot % full_lanes());
                        assert_eq!(
                            rows[row].get_full_step(lane),
                            op.step,
                            "step of virtual row {slot}"
                        );
                        assert_eq!(
                            rows[row].get_full_pc(lane),
                            index as u8,
                            "pc of virtual row {slot}"
                        );
                        assert_eq!(
                            rows[row].get_full_addr(lane),
                            op.addr,
                            "addr of virtual row {slot}"
                        );
                        slot += 1;
                    }
                }

                for slot in used..(rows.len() * full_lanes()) {
                    let (row, lane) = (slot / full_lanes(), slot % full_lanes());
                    assert!(
                        rows[row].get_full_reset(lane),
                        "padding lane {slot} is not a reset row"
                    );
                    assert_eq!(
                        rows[row].get_full_step(lane),
                        0,
                        "padding lane {slot} is not zeroed"
                    );
                    assert_eq!(
                        rows[row].get_full_addr(lane),
                        0,
                        "padding lane {slot} is not zeroed"
                    );
                    assert_eq!(
                        rows[row].get_full_reg(lane, 0),
                        0,
                        "padding lane {slot} is not zeroed"
                    );
                }
            }

            /// The byte operations land one per virtual row, which means side by side on the same
            /// row, and what follows them is the zero row.
            #[test]
            fn the_bytes_block_lands_one_operation_per_virtual_row() {
                let ops: Vec<MemAlignInput> =
                    (0..5).map(|i| byte_op_of(i + 1, i % 2 == 0)).collect();
                let ops: Vec<&MemAlignInput> = ops.iter().collect();

                let mut rows = vec![Fused::default(); 4];
                let (used, _) = fill_bytes_block_rows(&ops, &mut rows);
                assert_eq!(used, ops.len());

                for (slot, op) in ops.iter().enumerate() {
                    let (row, lane) = (slot / bytes_lanes(), slot % bytes_lanes());
                    assert_eq!(
                        rows[row].get_bytes_step(lane),
                        op.step,
                        "step of virtual row {slot}"
                    );
                    assert_eq!(
                        rows[row].get_bytes_addr_w(lane),
                        op.addr >> 3,
                        "address of virtual row {slot}"
                    );
                    assert_eq!(
                        rows[row].get_bytes_is_write(lane),
                        op.is_write,
                        "direction of virtual row {slot}"
                    );
                }

                for slot in used..(rows.len() * bytes_lanes()) {
                    let (row, lane) = (slot / bytes_lanes(), slot % bytes_lanes());
                    assert_eq!(
                        rows[row].get_bytes_step(lane),
                        0,
                        "padding lane {slot} is not zeroed"
                    );
                    assert_eq!(
                        rows[row].get_bytes_addr_w(lane),
                        0,
                        "padding lane {slot} is not zeroed"
                    );
                    assert!(
                        !rows[row].get_bytes_is_write(lane),
                        "padding lane {slot} is not zeroed"
                    );
                    assert_eq!(
                        rows[row].get_bytes_bus_byte(lane),
                        0,
                        "padding lane {slot} is not zeroed"
                    );
                }
            }

            /// The `full_` fill -- its padding included -- leaves the `bytes_` block exactly as it
            /// was.
            #[test]
            fn the_full_fill_leaves_the_bytes_block_untouched() {
                let ops: Vec<MemAlignInput> = [5usize, 2, 3]
                    .iter()
                    .enumerate()
                    .map(|(i, &r)| op_of(r, i as u64 + 1))
                    .collect();
                let ops: Vec<&MemAlignInput> = ops.iter().collect();

                // More rows than the ten virtual rows the operations take, so the padding runs too.
                let mut rows = vec![Fused::default(); 13];
                poison_bytes_block(&mut rows);
                let before = bytes_block_snapshot(&rows);

                fill_full_block_rows(&ops, &mut rows, &stamp);

                assert_eq!(
                    before,
                    bytes_block_snapshot(&rows),
                    "the full fill wrote into the bytes block"
                );
            }

            /// And the other way round, which is what lets the two fills run over the same rows in
            /// any order.
            #[test]
            fn the_bytes_fill_leaves_the_full_block_untouched() {
                let ops: Vec<MemAlignInput> =
                    (0..3).map(|i| byte_op_of(i + 1, i % 2 == 1)).collect();
                let ops: Vec<&MemAlignInput> = ops.iter().collect();

                let mut rows = vec![Fused::default(); 8];
                poison_full_block(&mut rows);
                let before = full_block_snapshot(&rows);

                fill_bytes_block_rows(&ops, &mut rows);

                assert_eq!(
                    before,
                    full_block_snapshot(&rows),
                    "the bytes fill wrote into the full block"
                );
            }
        }
    };
}

fill_tests_for!(compact_mem_align, CompactMemAlignTraceRow);
fill_tests_for!(compact_mem_align_large, CompactMemAlignLargeTraceRow);
