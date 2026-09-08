//! Equivalence of the parallel `Mem` fill against the one-range fill.
//!
//! This is the safety net for splitting the fill across threads. The split is what these tests
//! exercise: the fill body is the same code whatever the range count, so what has to be proved is
//! that cutting the work into `k` ranges produces byte-for-byte what one range produces — every
//! trace column of every lane, both multiplicity histograms, and the scalars the air values are
//! built from.
//!
//! What they do NOT prove is that the fill body itself is right: it was moved into `fill_mem_range`
//! unchanged (only `trace[row]` became `rows.at(row)`, `current_offsets` gained the range's base,
//! the slot bound became the range's, and operations outside the range's addresses are skipped), so
//! a one-range fill is the old sequential fill.
//!
//! Every case is run twice, once with the operations in address order and once **shuffled**.
//! `mem_ops` arrives unsorted in the offsets path -- the only code that sorts it is gated behind
//! `legacy_mem_count_and_plan` -- so a test that only ever fed sorted operations would miss a split
//! that assumed order. That is exactly the bug this pair of runs exists to catch.

use super::*;
use proofman_fields::Goldilocks;

type Row = MemTraceRow<Goldilocks>;

fn lanes_x_row() -> usize {
    Row::default().get_all_addr().len()
}

/// A segment whose addresses take `slots_per_addr` slots each, and the ops that fill them.
///
/// The ops are one read followed by `slots_per_addr - 1` writes per address, with strictly
/// increasing steps. That is the shape whose slot accounting is exactly one slot per op: the read
/// opens the address's first slot, and each write takes a new lane (`dual_available` is set, so the
/// write path advances `islot`). Reads would instead merge into the previous lane when they land in
/// the same chunk, which would make the offsets table depend on the chunk size — worth a test of
/// its own, but not what this one is about.
fn segment_and_ops(
    n_addrs: u32,
    slots_per_addr: u32,
) -> (MemModuleSegmentCheckPoint, Vec<MemInput>, u32) {
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
                o > 0, // the first access is a read, the rest are writes
                step,
                0xDEAD_0000_0000_0000 | ((a as u64) << 8) | o as u64,
            ));
            step += 1;
        }
    }
    (seg, ops, base)
}

/// Every witness column of every lane, flattened, so two fills can be compared exactly.
/// `previous_step` and `read_same_addr` are left out: they are `<==` columns in `mem.pil`, derived
/// by the prover, and the fill never writes them.
fn snapshot(rows: &[Row]) -> Vec<u64> {
    let lanes = lanes_x_row();
    let mut out = Vec::with_capacity(rows.len() * lanes * 11);
    for row in rows {
        for l in 0..lanes {
            out.push(row.get_addr(l) as u64);
            out.push(row.get_step(l));
            out.push(row.get_sel(l) as u64);
            out.push(row.get_addr_changes(l) as u64);
            out.push(row.get_wr(l) as u64);
            out.push(row.get_sel_dual(l) as u64);
            out.push(row.get_step_dual(l));
            out.push(row.get_value(l, 0) as u64);
            out.push(row.get_value(l, 1) as u64);
            out.push(row.get_l_increment(l) as u64);
            out.push(row.get_h_increment(l) as u64);
        }
    }
    out
}

/// Runs the fill over `chunks`, which stand in for the per-chunk vectors the collectors hand over.
/// Several chunks are worth using: `MemOps` chains them lazily, and a bug in that chaining would
/// only show with more than one.
fn run_chunks(
    n_rows: usize,
    seg: &MemModuleSegmentCheckPoint,
    chunks: &[Vec<MemInput>],
    n_ranges: usize,
) -> (Vec<u64>, MemFillOutput) {
    let mut rows = vec![Row::default(); n_rows];
    let prev = MemPreviousSegment { addr: RAM_W_ADDR_INIT, step: 0, value: 0 };
    let out = fill_mem_trace::<Goldilocks, Row>(
        &mut rows,
        MemOps::new(chunks),
        seg,
        &prev,
        SegmentId(0),
        true,
        n_ranges,
    );
    (snapshot(&rows), out)
}

/// The same operations as a single chunk.
fn run(
    n_rows: usize,
    seg: &MemModuleSegmentCheckPoint,
    ops: &[MemInput],
    n_ranges: usize,
) -> (Vec<u64>, MemFillOutput) {
    let one =
        vec![ops.iter().map(|o| MemInput::new(o.addr, o.is_write, o.step, o.value)).collect()];
    run_chunks(n_rows, seg, &one, n_ranges)
}

/// Splits `ops` into `n` chunks in order, the shape the collectors produce.
fn in_chunks(ops: &[MemInput], n: usize) -> Vec<Vec<MemInput>> {
    let per = ops.len().div_ceil(n).max(1);
    ops.chunks(per)
        .map(|c| c.iter().map(|o| MemInput::new(o.addr, o.is_write, o.step, o.value)).collect())
        .collect()
}

/// Deterministic shuffle: a fixed multiplier walk, so a failure is reproducible. Operations of the
/// same address keep their relative order, which is what the fill needs (it reads the previous
/// slot's step); what moves is the order the addresses are interleaved in, which is what a real
/// unsorted `mem_ops` looks like.
fn shuffled(ops: &[MemInput]) -> Vec<MemInput> {
    let n = ops.len();
    // Stable by address: keep each address's operations in step order, interleave the addresses.
    // `MemInput` derives only `Debug`, so the operations are rebuilt rather than cloned.
    let mut by_addr: std::collections::BTreeMap<u32, Vec<(bool, u64, u64)>> = Default::default();
    for op in ops {
        by_addr.entry(op.addr).or_default().push((op.is_write, op.step, op.value));
    }
    let mut cursors: Vec<(u32, usize)> = by_addr.keys().map(|&a| (a, 0)).collect();
    let mut out = Vec::with_capacity(n);
    let mut state2 = 0x853C_49E6_748F_EA9Bu64;
    while !cursors.is_empty() {
        state2 = state2.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let pick = (state2 >> 33) as usize % cursors.len();
        let (addr, ref mut idx) = cursors[pick];
        let (is_write, step, value) = by_addr[&addr][*idx];
        out.push(MemInput::new(addr, is_write, step, value));
        *idx += 1;
        if *idx == by_addr[&addr].len() {
            cursors.remove(pick);
        }
    }
    assert_eq!(out.len(), n);
    out
}

fn assert_same_as_one_range(n_addrs: u32, slots_per_addr: u32, n_rows: usize) {
    let (seg, sorted_ops, _) = segment_and_ops(n_addrs, slots_per_addr);
    let mixed = shuffled(&sorted_ops);
    // Both orders: `mem_ops` arrives unsorted in the offsets path.
    assert_same_as_one_range_with(&seg, &sorted_ops, n_rows);
    assert_same_as_one_range_with(&seg, &mixed, n_rows);
}

fn assert_same_as_one_range_with(
    seg: &MemModuleSegmentCheckPoint,
    ops: &[MemInput],
    n_rows: usize,
) {
    let (want_rows, want) = run(n_rows, seg, ops, 1);

    for k in [2usize, 3, 4, 5, 8] {
        // A vacuous comparison would pass, so check the work really was cut up.
        let ranges = split_mem_slots(seg, n_rows * lanes_x_row(), k);
        assert!(ranges.len() > 1, "k={k}: the split produced a single range, nothing to compare");

        let (got_rows, got) = run(n_rows, seg, ops, k);
        assert_eq!(got_rows, want_rows, "k={k}: the trace differs from the one-range fill");
        assert_eq!(got.range_22bits, want.range_22bits, "k={k}: 22-bit multiplicities differ");
        assert_eq!(got.range_16bits, want.range_16bits, "k={k}: 16-bit multiplicities differ");
        assert_eq!(got.last_addr, want.last_addr, "k={k}");
        assert_eq!(got.last_step, want.last_step, "k={k}");
        assert_eq!(got.last_value, want.last_value, "k={k}");
        assert_eq!(got.distance_base, want.distance_base, "k={k}");
        assert_eq!(got.distance_end, want.distance_end, "k={k}");
    }
}

/// The headline property, on a segment whose cuts land inside physical rows: 12 addresses of 3
/// slots is 36 slots, so with 8 lanes per row almost every cut falls mid-row and exercises the
/// shared-row merge.
#[test]
fn many_ranges_fill_exactly_what_one_range_fills() {
    assert_same_as_one_range(12, 3, 5);
}

/// An odd number of slots per address, so the cuts land on different lanes of the shared rows than
/// above — the merge has to copy a different lane interval each time.
#[test]
fn cuts_landing_on_any_lane_still_match() {
    for slots_per_addr in 1..=5 {
        let n_addrs = 16;
        let slots = (n_addrs * slots_per_addr) as usize;
        let n_rows = slots.div_ceil(lanes_x_row()) + 1; // +1 so there is padding to fill too
        assert_same_as_one_range(n_addrs, slots_per_addr, n_rows);
    }
}

/// One address per lane exactly: every cut is row-aligned, so no range has a partial leading row
/// and the merge copies whole rows. The uniform path through the same code.
#[test]
fn row_aligned_cuts_still_match() {
    assert_same_as_one_range(16, lanes_x_row() as u32, 16 + 1);
}

/// A trace with far more rows than operations: the fill is a sliver and the rest is padding, which
/// is the case the parallel padding fill has to get right.
#[test]
fn mostly_padding_still_matches() {
    assert_same_as_one_range(8, 2, 32);
}

/// Fewer addresses than ranges: the split hands back fewer ranges and the fill must still agree.
#[test]
fn fewer_addresses_than_ranges_still_matches() {
    let (seg, sorted_ops, _) = segment_and_ops(3, 2);
    let mixed = shuffled(&sorted_ops);
    let n_rows = 4;
    for ops in [&sorted_ops, &mixed] {
        let (want_rows, want) = run(n_rows, &seg, ops, 1);
        for k in [4usize, 8, 16] {
            let (got_rows, got) = run(n_rows, &seg, ops, k);
            assert_eq!(got_rows, want_rows, "k={k}");
            assert_eq!(got.range_22bits, want.range_22bits, "k={k}");
            assert_eq!(got.range_16bits, want.range_16bits, "k={k}");
        }
    }
}

/// Chunked exactly as the collectors hand it over: several vectors, walked without being
/// concatenated. One chunk and many must fill the same trace, or the lazy chaining is wrong.
#[test]
fn many_chunks_fill_the_same_as_one() {
    let (seg, ops, _) = segment_and_ops(24, 3);
    let n_rows = 10;
    let (want_rows, want) = run(n_rows, &seg, &ops, 1);
    for n_chunks in [2usize, 3, 7, 24] {
        let chunks = in_chunks(&ops, n_chunks);
        assert!(chunks.len() > 1, "{n_chunks}: expected several chunks");
        for k in [1usize, 2, 4, 8] {
            let (rows, out) = run_chunks(n_rows, &seg, &chunks, k);
            assert_eq!(rows, want_rows, "{n_chunks} chunks, k={k}: different trace");
            assert_eq!(out.range_22bits, want.range_22bits, "{n_chunks} chunks, k={k}");
            assert_eq!(out.range_16bits, want.range_16bits, "{n_chunks} chunks, k={k}");
            assert_eq!(out.last_addr, want.last_addr, "{n_chunks} chunks, k={k}");
        }
    }
}

/// The regression this whole file failed to catch the first time: with the operations shuffled, a
/// split that assumed `mem_ops` was in address order writes outside its own rows. Kept as its own
/// test, with the shuffle checked to be a real one, so the intent cannot quietly be lost again.
#[test]
fn unsorted_operations_fill_the_same_as_sorted_ones() {
    let (seg, sorted_ops, _) = segment_and_ops(24, 3);
    let mixed = shuffled(&sorted_ops);
    assert_ne!(
        mixed.iter().map(|o| o.addr).collect::<Vec<_>>(),
        sorted_ops.iter().map(|o| o.addr).collect::<Vec<_>>(),
        "the shuffle must actually reorder the addresses"
    );
    let n_rows = 10;
    let (sorted_rows, sorted_out) = run(n_rows, &seg, &sorted_ops, 1);
    for k in [1usize, 2, 4, 8] {
        let (rows, out) = run(n_rows, &seg, &mixed, k);
        assert_eq!(rows, sorted_rows, "k={k}: unsorted input filled a different trace");
        assert_eq!(out.range_22bits, sorted_out.range_22bits, "k={k}");
        assert_eq!(out.range_16bits, sorted_out.range_16bits, "k={k}");
        assert_eq!(out.last_addr, sorted_out.last_addr, "k={k}");
    }
}

/// The fill must actually have written something, or every assertion above would be comparing two
/// empty traces. Pins the shape the other tests rely on: one slot per operation, `sel` set on the
/// filled slots and clear on the padding.
#[test]
fn the_fill_writes_one_slot_per_operation() {
    let (n_addrs, slots_per_addr) = (12u32, 3u32);
    let (seg, ops, base) = segment_and_ops(n_addrs, slots_per_addr);
    let lanes = lanes_x_row();
    let n_rows = 5;
    let mut rows = vec![Row::default(); n_rows];
    let prev = MemPreviousSegment { addr: RAM_W_ADDR_INIT, step: 0, value: 0 };
    let chunks = in_chunks(&ops, 3);
    let out = fill_mem_trace::<Goldilocks, Row>(
        &mut rows,
        MemOps::new(&chunks),
        &seg,
        &prev,
        SegmentId(0),
        true,
        4,
    );

    let filled = (n_addrs * slots_per_addr) as usize;
    for slot in 0..filled {
        let (r, l) = (slot / lanes, slot % lanes);
        assert!(rows[r].get_sel(l), "slot {slot} should be selected");
        assert_eq!(
            rows[r].get_addr(l),
            base + (slot as u32 / slots_per_addr),
            "slot {slot} holds the wrong address"
        );
    }
    for slot in filled..(n_rows * lanes) {
        let (r, l) = (slot / lanes, slot % lanes);
        assert!(!rows[r].get_sel(l), "padding slot {slot} must not be selected");
    }
    // The last filled slot is what the padding repeats and what the segment hands on.
    assert_eq!(out.last_addr, base + n_addrs - 1);
}
