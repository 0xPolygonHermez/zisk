//! Unit tests for `ArithEqPlanner::plan` — the filler that turns the per-air strategy into one
//! `Plan` per instance, with a per-(chunk, op) collect window.
//!
//! Declared from `arith_eq_family.rs` via `#[cfg(test)] #[path = …] mod tests;`.
//!
//! The property that matters is **tiling**: for every (chunk, op), the collect windows emitted
//! across all plans must partition that chunk's occurrences of that op — no overlap (an operation
//! proved twice) and no gap (an operation never proved). The strategy decides *where* rows go; this
//! is what guarantees the plans actually collect them exactly once.

use super::*;
use proofman_fields::Goldilocks;
use std::collections::BTreeMap;
use zisk_pil::ArithEqLargeTrace;

// NOTE: not named `Planner`, which would shadow the `zisk_common::Planner` trait `plan` comes from.
type TestPlanner = ArithEqPlanner<Goldilocks>;

fn cap_of(air_id: usize) -> u64 {
    let meta = air_metas().into_iter().find(|m| m.air_id == air_id).unwrap();
    meta.num_rows as u64 / ARITH_EQ_ROWS_BY_OP as u64
}

/// Builds the planner input from per-chunk `(op, count)` lists, in chunk order.
fn counters(per_chunk: &[&[(ArithEqOp, u64)]]) -> Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> {
    per_chunk
        .iter()
        .enumerate()
        .map(|(idx, ops)| {
            let mut counter = ArithEqCounterInputGen::<Goldilocks>::new(BusDeviceMode::Counter);
            for &(op, n) in *ops {
                counter.counts[op.index()] = n;
            }
            (ChunkId(idx), Box::new(counter) as Box<dyn BusDeviceMetrics>)
        })
        .collect()
}

fn checkpoint_of(plan: &Plan) -> &ArithEqCheckPoints {
    plan.meta.as_ref().unwrap().downcast_ref::<ArithEqCheckPoints>().unwrap()
}

/// Asserts the collect windows of every (chunk, op) tile `[0, count)` exactly once, and that no
/// instance is over-filled. Returns the number of plans per air id.
fn assert_tiles(plans: &[Plan], per_chunk: &[&[(ArithEqOp, u64)]]) -> BTreeMap<usize, usize> {
    // (chunk, op) -> the windows every plan asks to collect, as (skip, count).
    let mut windows: BTreeMap<(usize, usize), Vec<(u64, u64)>> = BTreeMap::new();
    let mut plans_per_air: BTreeMap<usize, usize> = BTreeMap::new();

    for plan in plans {
        *plans_per_air.entry(plan.air_id).or_default() += 1;
        let checkpoint = checkpoint_of(plan);
        assert!(!checkpoint.is_empty(), "air {} emitted an empty instance", plan.air_id);

        // The plan's CheckPoint must list exactly the chunks its metadata covers, or the executor
        // would build a collector for a chunk with no window (or miss one entirely).
        let listed = match &plan.check_point {
            CheckPoint::Multiple(chunks) => chunks.to_vec(),
            other => panic!("expected CheckPoint::Multiple, got {other:?}"),
        };
        assert_eq!(listed, checkpoint.chunk_ids(), "chunk list and metadata windows disagree");
        // `get` is the binary search the executor uses to find a chunk's window; it must resolve
        // every chunk the plan listed, which also asserts the sorted invariant holds.
        for &chunk_id in &listed {
            assert_eq!(checkpoint.get(chunk_id).chunk_id, chunk_id);
        }

        let mut filled = 0u64;
        for cp in checkpoint.iter() {
            let chunk_id = &cp.chunk_id;
            assert_eq!(cp.air_id, plan.air_id);
            for (op_idx, counter) in cp.ops.iter().enumerate() {
                if counter.collect_count == 0 {
                    continue;
                }
                filled += counter.collect_count as u64;
                windows
                    .entry((chunk_id.0, op_idx))
                    .or_default()
                    .push((counter.initial_skip as u64, counter.collect_count as u64));
            }
        }
        assert!(
            filled <= cap_of(plan.air_id),
            "air {} instance holds {filled} ops, capacity is {}",
            plan.air_id,
            cap_of(plan.air_id)
        );
    }

    // Every op occurrence of every chunk must be covered exactly once.
    for (chunk_idx, ops) in per_chunk.iter().enumerate() {
        for &(op, count) in *ops {
            if count == 0 {
                continue;
            }
            let key = (chunk_idx, op.index());
            let mut got = windows.remove(&key).unwrap_or_default();
            got.sort_unstable();

            let mut covered = 0u64;
            for (skip, len) in &got {
                assert_eq!(
                    *skip, covered,
                    "chunk {chunk_idx} op {op:?}: window starts at {skip}, expected {covered} \
                     (windows: {got:?})"
                );
                covered += len;
            }
            assert_eq!(
                covered, count,
                "chunk {chunk_idx} op {op:?}: windows cover {covered} of {count} occurrences \
                 (windows: {got:?})"
            );
        }
    }
    // Anything left refers to an op/chunk that was never counted.
    assert!(windows.is_empty(), "windows for uncounted (chunk, op) pairs: {windows:?}");

    plans_per_air
}

fn plan_of(per_chunk: &[&[(ArithEqOp, u64)]]) -> Vec<Plan> {
    TestPlanner::new().plan(counters(per_chunk))
}

/// No instance may be handed more operations than *its own* air holds. The configs come in two
/// heights, so a plan sized against the shorter one would be rejected by the witness computation the
/// moment it landed on a tall instance — which is exactly the shape of the capacity bug this pins.
#[test]
fn no_instance_is_given_more_than_its_air_holds() {
    let tall = cap_of(ArithEqLargeTrace::<()>::AIR_ID);
    let shapes: &[&[(ArithEqOp, u64)]] = &[
        &[(ArithEqOp::Arith256, 3)],
        &[(ArithEqOp::Arith256, tall + 1)],
        &[(ArithEqOp::Secp256k1Add, 2 * tall + 10), (ArithEqOp::Arith256Mod, 7)],
        &[(ArithEqOp::Secp256r1Add, tall / 2), (ArithEqOp::Bn254ComplexMul, tall / 2)],
    ];

    for shape in shapes {
        let per_chunk: &[&[(ArithEqOp, u64)]] = &[shape];
        for plan in plan_of(per_chunk) {
            let collected: u64 = checkpoint_of(&plan).iter().map(|cp| cp.count() as u64).sum();
            let capacity = cap_of(plan.air_id);
            assert!(
                collected <= capacity,
                "air {} was given {collected} operations but holds {capacity}",
                plan.air_id,
            );
        }
    }
}

#[test]
fn no_operations_plans_nothing() {
    assert!(plan_of(&[&[], &[]]).is_empty());
}

#[test]
fn single_chunk_single_op() {
    let per_chunk: &[&[(ArithEqOp, u64)]] = &[&[(ArithEqOp::Arith256, 3)]];
    let plans = plan_of(per_chunk);
    let per_air = assert_tiles(&plans, per_chunk);
    assert_eq!(per_air.values().sum::<usize>(), 1);
}

#[test]
fn windows_tile_across_several_chunks() {
    let per_chunk: &[&[(ArithEqOp, u64)]] = &[
        &[(ArithEqOp::Arith256, 5), (ArithEqOp::Secp256k1Add, 2)],
        &[(ArithEqOp::Arith256, 7)],
        &[(ArithEqOp::Secp256k1Add, 4), (ArithEqOp::Bn254ComplexMul, 1)],
    ];
    let plans = plan_of(per_chunk);
    assert_tiles(&plans, per_chunk);
}

#[test]
fn an_op_spanning_several_instances_is_split_without_gaps() {
    // One chunk holding more of a single op than an instance can take: the filler must cut it into
    // consecutive windows, one per instance.
    // The tall universal air is where a bulk of any operation goes: it holds the most per instance.
    let air = ArithEqLargeTrace::<()>::AIR_ID;
    let cap = cap_of(air);
    let per_chunk: &[&[(ArithEqOp, u64)]] = &[&[(ArithEqOp::Arith256, 2 * cap + 10)]];
    let plans = plan_of(per_chunk);
    let per_air = assert_tiles(&plans, per_chunk);

    assert_eq!(
        per_air.values().sum::<usize>(),
        3,
        "expected ceil((2·cap + 10)/cap) = 3 instances, the tail possibly in a shorter air"
    );
    assert_eq!(per_air.get(&air), Some(&2), "the two full instances stay in the tall air");
}

#[test]
fn an_instance_boundary_inside_a_chunk_keeps_both_windows() {
    // More operations than one instance holds, so an instance fills up midway through a chunk and
    // that chunk appears in two instances with disjoint windows for the same op. Which air each
    // instance belongs to is the strategy's business — what the filler owes is that the windows
    // still tile.
    let cap = cap_of(ArithEqLargeTrace::<()>::AIR_ID);
    let per_chunk: &[&[(ArithEqOp, u64)]] =
        &[&[(ArithEqOp::Arith256, cap - 1)], &[(ArithEqOp::Arith256, 5)]];
    let plans = plan_of(per_chunk);
    assert_tiles(&plans, per_chunk);
    assert_eq!(plans.len(), 2, "cap + 4 operations need two instances");

    // Exactly one chunk is straddled, and its two windows meet without a gap or an overlap.
    let op = ArithEqOp::Arith256.index();
    let straddled: Vec<usize> = (0..per_chunk.len())
        .filter(|&chunk| {
            plans
                .iter()
                .filter(|p| checkpoint_of(p).iter().any(|cp| cp.chunk_id == ChunkId(chunk)))
                .count()
                > 1
        })
        .collect();
    assert_eq!(straddled.len(), 1, "one instance boundary falls inside one chunk");

    let mut windows: Vec<(u32, u32)> = plans
        .iter()
        .filter_map(|p| checkpoint_of(p).iter().find(|cp| cp.chunk_id == ChunkId(straddled[0])))
        .map(|cp| (cp.ops[op].initial_skip, cp.ops[op].collect_count))
        .collect();
    windows.sort();
    assert_eq!(windows[0].0, 0, "the first window starts at the beginning of the chunk");
    assert_eq!(windows[1].0, windows[0].1, "and the second picks up exactly where it stopped");
}

#[test]
fn segment_ids_are_contiguous_per_air() {
    let air = ArithEqLargeTrace::<()>::AIR_ID;
    let cap = cap_of(air);
    let per_chunk: &[&[(ArithEqOp, u64)]] =
        &[&[(ArithEqOp::Arith256, 2 * cap + 1), (ArithEqOp::Secp256k1Add, 1)]];
    let plans = plan_of(per_chunk);
    assert_tiles(&plans, per_chunk);

    let mut seen: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for plan in &plans {
        seen.entry(plan.air_id).or_default().push(plan.segment_id.unwrap().0);
    }
    for (air_id, mut segments) in seen {
        segments.sort_unstable();
        let expected: Vec<usize> = (0..segments.len()).collect();
        assert_eq!(segments, expected, "air {air_id} segment ids must be 0..n");
    }
}

#[test]
fn a_split_op_tiles_across_two_airs() {
    // A family big enough to fill whole instances of the tall universal air, plus a remainder worth
    // pooling elsewhere: the same op is then proved partly by one air and partly by another, and the
    // windows must still tile.
    let tall = ArithEqLargeTrace::<()>::AIR_ID;
    let cap = cap_of(tall);
    let per_chunk: &[&[(ArithEqOp, u64)]] =
        &[&[(ArithEqOp::Secp256k1Add, 2 * cap + 10), (ArithEqOp::Arith256Mod, 7)]];
    let plans = plan_of(per_chunk);
    let per_air = assert_tiles(&plans, per_chunk);

    // The bulk fills two instances of the tall air, which is where the fewest of them are needed.
    assert_eq!(per_air.get(&tall), Some(&2));

    // And the split the test is named for: the 10-op tail is proved by a different air. Its window
    // and the bulk's must meet without a gap, whichever of the two the filler emitted first.
    let op = ArithEqOp::Secp256k1Add.index();
    let mut windows: Vec<(u64, u64)> = plans
        .iter()
        .map(|p| checkpoint_of(p).get(ChunkId(0)).ops[op])
        .filter(|w| w.collect_count > 0)
        .map(|w| (w.initial_skip as u64, w.collect_count as u64))
        .collect();
    windows.sort();
    assert_eq!(windows.len(), 3, "two bulk instances and the pooled tail");

    let tail = plans
        .iter()
        .find(|p| p.air_id != tall && checkpoint_of(p).get(ChunkId(0)).ops[op].collect_count > 0)
        .expect("the tail must be pooled into another air");
    assert_eq!(checkpoint_of(tail).get(ChunkId(0)).ops[op].collect_count as u64, 10);
}

#[test]
fn every_op_is_planned_even_without_a_specialized_air() {
    // secp256r1 has no specialized air, so it can only be proved by the universal one; it must
    // still be covered.
    let per_chunk: &[&[(ArithEqOp, u64)]] =
        &[&[(ArithEqOp::Secp256r1Add, 3), (ArithEqOp::Secp256r1Dbl, 2), (ArithEqOp::Arith256, 4)]];
    let plans = plan_of(per_chunk);
    assert_tiles(&plans, per_chunk);
}

#[test]
fn all_eleven_ops_at_once_tile() {
    let all: Vec<(ArithEqOp, u64)> =
        ArithEqOp::ALL.iter().enumerate().map(|(i, &op)| (op, i as u64 + 1)).collect();
    let per_chunk: &[&[(ArithEqOp, u64)]] = &[&all];
    let plans = plan_of(per_chunk);
    assert_tiles(&plans, per_chunk);
}
