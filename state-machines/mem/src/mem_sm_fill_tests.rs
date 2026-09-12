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

/// Rows a segment of `n_addrs * slots_per_addr` slots needs, plus one so there is always padding to
/// fill. Derived from the lane count because `zisk.pil` changes it: a hardcoded row count silently
/// gives a trace too small for the operations, and the fill then stops at its slot limit.
fn rows_for(n_addrs: u32, slots_per_addr: u32) -> usize {
    ((n_addrs * slots_per_addr) as usize).div_ceil(lanes_x_row()) + 1
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
    assert_same_as_one_range(12, 3, rows_for(12, 3));
}

/// An odd number of slots per address, so the cuts land on different lanes of the shared rows than
/// above — the merge has to copy a different lane interval each time.
#[test]
fn cuts_landing_on_any_lane_still_match() {
    for slots_per_addr in 1..=5 {
        let n_addrs = 16;
        assert_same_as_one_range(n_addrs, slots_per_addr, rows_for(n_addrs, slots_per_addr));
    }
}

/// One address per lane exactly: every cut is row-aligned, so no range has a partial leading row
/// and the merge copies whole rows. The uniform path through the same code.
#[test]
fn row_aligned_cuts_still_match() {
    let per_addr = lanes_x_row() as u32;
    assert_same_as_one_range(16, per_addr, rows_for(16, per_addr));
}

/// A trace with far more rows than operations: the fill is a sliver and the rest is padding, which
/// is the case the parallel padding fill has to get right.
#[test]
fn mostly_padding_still_matches() {
    // Far more rows than the operations need, so most of the trace is padding.
    assert_same_as_one_range(8, 2, rows_for(8, 2) * 8);
}

/// Fewer addresses than ranges: the split hands back fewer ranges and the fill must still agree.
#[test]
fn fewer_addresses_than_ranges_still_matches() {
    let (seg, sorted_ops, _) = segment_and_ops(3, 2);
    let mixed = shuffled(&sorted_ops);
    let n_rows = rows_for(3, 2);
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
    let n_rows = rows_for(24, 3);
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
    let n_rows = rows_for(24, 3);
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
    let n_rows = rows_for(n_addrs, slots_per_addr);
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

// ---------------------------------------------------------------------------------------------
// The trace buffer is NOT zeroed before the fill
// ---------------------------------------------------------------------------------------------
//
// `compute_witness_with_offsets_inner` takes its buffer from the recycled basic-trace pool with
// `new_from_vec`, so every lane the fill and the padding leave untouched keeps the *previous*
// instance's bytes. These tests pin what makes that sound: the fill plus the padding assign every
// column of every lane, except the two the prover derives from its `witness_calc` hints
// (`previous_step` and `read_same_addr` -- see `mem.pil`, and `calculateWitnessExpr` in
// pil2-stark, which runs before both the contribution commit and the stage-1 commit).
//
// The method is differential: run the same fill twice, once over a zeroed buffer and once over a
// buffer poisoned with a value no fill would ever produce. Any column left unwritten shows up as a
// difference. Comparing against a poison rather than asserting a fixed expected trace is what
// makes the test survive changes to the fill itself.

/// Every column of every lane set to a value the fill cannot produce: addresses far outside the
/// segment, steps and values with every byte set. A lane left unwritten keeps these.
fn poison_row() -> Row {
    let mut row = Row::default();
    for lane in 0..lanes_x_row() {
        row.set_addr(lane, 0x1FFF_FFFF);
        row.set_step(lane, (1u64 << 38) - 1);
        row.set_sel(lane, true);
        row.set_addr_changes(lane, true);
        row.set_sel_dual(lane, true);
        row.set_step_dual(lane, (1u64 << 38) - 1);
        row.set_value(lane, 0, u32::MAX);
        row.set_value(lane, 1, u32::MAX);
        row.set_wr(lane, true);
        row.set_l_increment(lane, (1 << 22) - 1);
        row.set_h_increment(lane, u16::MAX);
        // The two the fill never writes, so a run over this buffer must still show them poisoned.
        row.set_previous_step(lane, (1u64 << 40) - 1);
        row.set_read_same_addr(lane, true);
    }
    row
}

/// Like [`snapshot`] but including the two prover-derived columns, so a difference can be
/// attributed to a named column rather than to an opaque index.
fn snapshot_all(rows: &[Row]) -> Vec<(usize, usize, &'static str, u64)> {
    let lanes = lanes_x_row();
    let mut out = Vec::with_capacity(rows.len() * lanes * 13);
    for (r, row) in rows.iter().enumerate() {
        for l in 0..lanes {
            out.push((r, l, "addr", row.get_addr(l) as u64));
            out.push((r, l, "step", row.get_step(l)));
            out.push((r, l, "sel", row.get_sel(l) as u64));
            out.push((r, l, "addr_changes", row.get_addr_changes(l) as u64));
            out.push((r, l, "wr", row.get_wr(l) as u64));
            out.push((r, l, "sel_dual", row.get_sel_dual(l) as u64));
            out.push((r, l, "step_dual", row.get_step_dual(l)));
            out.push((r, l, "value0", row.get_value(l, 0) as u64));
            out.push((r, l, "value1", row.get_value(l, 1) as u64));
            out.push((r, l, "l_increment", row.get_l_increment(l) as u64));
            out.push((r, l, "h_increment", row.get_h_increment(l) as u64));
            out.push((r, l, "previous_step", row.get_previous_step(l)));
            out.push((r, l, "read_same_addr", row.get_read_same_addr(l) as u64));
        }
    }
    out
}

/// Runs the fill over a caller-supplied buffer, so the same operations can be filled into a zeroed
/// trace and into a poisoned one.
fn run_over(
    mut rows: Vec<Row>,
    seg: &MemModuleSegmentCheckPoint,
    chunks: &[Vec<MemInput>],
    n_ranges: usize,
) -> (Vec<Row>, MemFillOutput) {
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
    (rows, out)
}

/// The columns the prover fills in from its `witness_calc` hints, and which the witness therefore
/// must NOT be expected to assign.
const PROVER_DERIVED: [&str; 2] = ["previous_step", "read_same_addr"];

/// Fills the same operations into a zeroed buffer and into a poisoned one and reports which
/// columns came out different. Every name it returns is a column the fill left unwritten.
fn columns_left_unwritten(
    seg: &MemModuleSegmentCheckPoint,
    ops: &[MemInput],
    n_rows: usize,
    n_ranges: usize,
) -> Vec<String> {
    let chunks = in_chunks(ops, 3);
    let (clean_rows, clean_out) = run_over(vec![Row::default(); n_rows], seg, &chunks, n_ranges);
    let (dirty_rows, dirty_out) = run_over(vec![poison_row(); n_rows], seg, &chunks, n_ranges);

    // The scalars the air values are built from must not depend on the buffer either: they are
    // read back off the trace, so an unwritten lane would leak into the continuation.
    assert_eq!(dirty_out.last_addr, clean_out.last_addr, "last_addr depends on the buffer");
    assert_eq!(dirty_out.last_step, clean_out.last_step, "last_step depends on the buffer");
    assert_eq!(dirty_out.last_value, clean_out.last_value, "last_value depends on the buffer");
    assert_eq!(dirty_out.distance_base, clean_out.distance_base, "distance_base");
    assert_eq!(dirty_out.distance_end, clean_out.distance_end, "distance_end");
    assert_eq!(dirty_out.range_22bits, clean_out.range_22bits, "22-bit multiplicities");
    assert_eq!(dirty_out.range_16bits, clean_out.range_16bits, "16-bit multiplicities");

    let mut differing: Vec<String> = Vec::new();
    for (clean, dirty) in snapshot_all(&clean_rows).iter().zip(snapshot_all(&dirty_rows).iter()) {
        if clean.3 != dirty.3 && !differing.iter().any(|c| c == clean.2) {
            differing.push(format!("{} (row {} lane {})", clean.2, clean.0, clean.1));
        }
    }
    differing
}

fn assert_only_prover_derived_are_left(
    seg: &MemModuleSegmentCheckPoint,
    ops: &[MemInput],
    n_rows: usize,
    n_ranges: usize,
    what: &str,
) {
    let left = columns_left_unwritten(seg, ops, n_rows, n_ranges);
    let unexpected: Vec<&String> =
        left.iter().filter(|c| !PROVER_DERIVED.iter().any(|d| c.starts_with(d))).collect();
    assert!(
        unexpected.is_empty(),
        "{what}: the fill left these columns unwritten, so the trace depends on the recycled \
         buffer: {unexpected:?}"
    );
}

/// The headline property: over every shape the other tests use, the only columns whose value
/// depends on what the buffer held are the two the prover derives.
#[test]
fn the_fill_assigns_every_column_the_prover_does_not_derive() {
    let shapes: [(u32, u32, usize); 6] = [
        // (addresses, slots per address, row multiplier)
        (12, 3, 1), // cuts land mid-row
        (16, 1, 1), // one slot per address
        (16, 5, 1), // several slots per address
        (8, 2, 8),  // mostly padding: one sliver of fill, the rest padded
        (3, 2, 1),  // fewer addresses than ranges
        (24, 3, 1), // the shape the chunked tests use
    ];
    for (n_addrs, slots_per_addr, mult) in shapes {
        let (seg, sorted_ops, _) = segment_and_ops(n_addrs, slots_per_addr);
        let mixed = shuffled(&sorted_ops);
        let n_rows = rows_for(n_addrs, slots_per_addr) * mult;
        for (label, ops) in [("sorted", &sorted_ops), ("shuffled", &mixed)] {
            for k in [1usize, 2, 4, 8] {
                assert_only_prover_derived_are_left(
                    &seg,
                    ops,
                    n_rows,
                    k,
                    &format!("{n_addrs}x{slots_per_addr} rows*{mult} {label} k={k}"),
                );
            }
        }
    }
}

/// The test above only means something if the poison is actually visible: the two prover-derived
/// columns must come out of the poisoned run still poisoned. If they did not, the comparison would
/// be passing because the poison never reached the trace, not because the fill wrote everything.
#[test]
fn the_poison_really_reaches_the_trace() {
    let (seg, ops, _) = segment_and_ops(12, 3);
    let left = columns_left_unwritten(&seg, &ops, rows_for(12, 3), 4);
    for derived in PROVER_DERIVED {
        assert!(
            left.iter().any(|c| c.starts_with(derived)),
            "{derived} came out of the poisoned buffer unchanged, so the poison never reached the \
             trace and `the_fill_assigns_every_column_the_prover_does_not_derive` is vacuous"
        );
    }
}

/// A segment whose padding starts mid-row exercises the one-lane-at-a-time branch of the padding,
/// which writes into a row the fill already touched. That row is where an unwritten padding column
/// would hide: the whole-row branch builds its row from `R::default()` and so cannot leak, while
/// this one writes into the recycled buffer.
#[test]
fn padding_that_starts_mid_row_assigns_every_lane() {
    let lanes = lanes_x_row() as u32;
    // One slot short of a whole row of operations, so the padding starts on the last lane.
    let n_addrs = lanes * 2 - 1;
    let (seg, ops, _) = segment_and_ops(n_addrs, 1);
    assert_ne!(
        (n_addrs % lanes),
        0,
        "the operations must not end on a row boundary or the partial padding row is never built"
    );
    for k in [1usize, 2, 4] {
        assert_only_prover_derived_are_left(
            &seg,
            &ops,
            rows_for(n_addrs, 1) + 2,
            k,
            &format!("partial padding row, k={k}"),
        );
    }
}

/// No operations at all. The planner does not produce such a segment, but the padding used to seed
/// itself from slot 0 -- a slot nobody had written -- so on a recycled buffer it would have padded
/// the whole trace with the previous instance's address and step. It must fall back to the
/// hand-over from the previous segment instead.
#[test]
fn an_empty_segment_pads_from_the_previous_segment() {
    let seg = MemModuleSegmentCheckPoint::default();
    let n_rows = 4;
    let lanes = lanes_x_row();
    let prev =
        MemPreviousSegment { addr: RAM_W_ADDR_INIT + 7, step: 42, value: 0xABCD_1234_5678_9ABC };

    let mut rows = vec![poison_row(); n_rows];
    let out = fill_mem_trace::<Goldilocks, Row>(
        &mut rows,
        MemOps::new(&[Vec::new()]),
        &seg,
        &prev,
        SegmentId(0),
        true,
        4,
    );

    assert_eq!(out.last_addr, prev.addr, "the empty segment must hand on the previous address");
    assert_eq!(out.last_step, prev.step, "the empty segment must hand on the previous step");
    for (r, row) in rows.iter().enumerate() {
        for l in 0..lanes {
            assert_eq!(row.get_addr(l), prev.addr, "row {r} lane {l}: addr");
            assert_eq!(row.get_step(l), prev.step, "row {r} lane {l}: step");
            assert!(!row.get_sel(l), "row {r} lane {l}: padding must not be selected");
            assert!(!row.get_sel_dual(l), "row {r} lane {l}: sel_dual");
            assert_eq!(row.get_value(l, 0), prev.value as u32, "row {r} lane {l}: value low");
            assert_eq!(
                row.get_value(l, 1),
                (prev.value >> 32) as u32,
                "row {r} lane {l}: value high"
            );
        }
    }
}
