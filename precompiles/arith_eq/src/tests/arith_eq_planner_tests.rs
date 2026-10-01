//! Unit tests for the ArithEq air-selection planner (moved out of `arith_eq_planner.rs`).
//! Declared there via `#[cfg(test)] #[path = "tests/arith_eq_planner_tests.rs"] mod tests;`,
//! so it stays a child module of `arith_eq_planner` and keeps `super::` access to privates.

use super::*;
use zisk_pil::{
    Arith256XBeLargeTrace, Arith256XBeTrace, Arith256XLargeTrace, Arith256XTrace,
    ArithBn254BeLargeTrace, ArithBn254BeTrace, ArithBn254LargeTrace, ArithBn254Trace,
    ArithEqBeLargeTrace, ArithEqBeTrace, ArithEqLargeTrace, ArithEqTrace,
    ArithSecp256K1BeLargeTrace, ArithSecp256K1BeTrace, ArithSecp256K1LargeTrace,
    ArithSecp256K1Trace,
};

fn counts(pairs: &[(ArithEqOp, u64)]) -> [u64; ARITH_EQ_OP_NUM] {
    let mut c = [0u64; ARITH_EQ_OP_NUM];
    for &(op, n) in pairs {
        c[op.index()] = n;
    }
    c
}

fn meta_of(air_id: usize) -> ArithEqAirMeta {
    air_metas().into_iter().find(|m| m.air_id == air_id).unwrap()
}

fn plan_area(plan: &[ArithEqAirPlan]) -> u64 {
    plan.iter().map(|p| p.instances * meta_of(p.air_id).cost as u64).sum()
}

fn plan_instances(plan: &[ArithEqAirPlan]) -> u64 {
    plan.iter().map(|p| p.instances).sum()
}

// Config-air ids taken straight from the generated `zisk_pil` trace types (pil_helpers/traces.rs),
// the pilout source of truth — so the test tracks AIR_ID renumbering automatically and validates the
// planner against the real airs rather than against `air_metas()`.
fn all_air_ids() -> Vec<usize> {
    vec![
        ArithEqTrace::<()>::AIR_ID,
        ArithEqLargeTrace::<()>::AIR_ID,
        Arith256XTrace::<()>::AIR_ID,
        Arith256XLargeTrace::<()>::AIR_ID,
        ArithSecp256K1Trace::<()>::AIR_ID,
        ArithSecp256K1LargeTrace::<()>::AIR_ID,
        ArithBn254Trace::<()>::AIR_ID,
        ArithBn254LargeTrace::<()>::AIR_ID,
        ArithEqBeTrace::<()>::AIR_ID,
        ArithEqBeLargeTrace::<()>::AIR_ID,
        Arith256XBeTrace::<()>::AIR_ID,
        Arith256XBeLargeTrace::<()>::AIR_ID,
        ArithSecp256K1BeTrace::<()>::AIR_ID,
        ArithSecp256K1BeLargeTrace::<()>::AIR_ID,
        ArithBn254BeTrace::<()>::AIR_ID,
        ArithBn254BeLargeTrace::<()>::AIR_ID,
    ]
}

/// The big-endian air ids, which cover the `*Be` operations only.
fn big_endian_air_ids() -> Vec<usize> {
    all_air_ids()[8..].to_vec()
}

fn assert_conserves(plan: &[ArithEqAirPlan], totals: &[u64; ARITH_EQ_OP_NUM]) {
    let mut summed = [0u64; ARITH_EQ_OP_NUM];
    for p in plan {
        for (s, c) in summed.iter_mut().zip(p.op_counts.iter()) {
            *s += *c;
        }
    }
    assert_eq!(&summed, totals, "planner must conserve every operation's total count");
}

/// Every config must be a size ladder: each step strictly taller than the one below, exactly as
/// wide, and covering the same operations. That is what the strategy relies on to trade memory for a
/// lower instance count. The ladders are walked pairwise rather than assumed to be pairs, so a
/// config that gains a third height in `zisk.pil` is still checked step by step.
#[test]
fn every_config_is_a_size_ladder() {
    for ladder in [
        &[ArithEqTrace::<()>::AIR_ID, ArithEqLargeTrace::<()>::AIR_ID][..],
        &[Arith256XTrace::<()>::AIR_ID, Arith256XLargeTrace::<()>::AIR_ID][..],
        &[ArithSecp256K1Trace::<()>::AIR_ID, ArithSecp256K1LargeTrace::<()>::AIR_ID][..],
        &[ArithBn254Trace::<()>::AIR_ID, ArithBn254LargeTrace::<()>::AIR_ID][..],
        &[ArithEqBeTrace::<()>::AIR_ID, ArithEqBeLargeTrace::<()>::AIR_ID][..],
        &[Arith256XBeTrace::<()>::AIR_ID, Arith256XBeLargeTrace::<()>::AIR_ID][..],
        &[ArithSecp256K1BeTrace::<()>::AIR_ID, ArithSecp256K1BeLargeTrace::<()>::AIR_ID][..],
        &[ArithBn254BeTrace::<()>::AIR_ID, ArithBn254BeLargeTrace::<()>::AIR_ID][..],
    ] {
        for step in ladder.windows(2) {
            let (short, tall) = (meta_of(step[0]), meta_of(step[1]));
            assert!(tall.num_rows > short.num_rows, "air {} must be taller", tall.air_id);
            assert!(tall.cost > short.cost, "air {} must cost more, being taller", tall.air_id);
            assert_eq!(tall.ops, short.ops, "air {} must cover the same operations", tall.air_id);
        }
    }
}

/// The sweep is exhaustive over which air takes each tail, so its cost is the product of the
/// candidate counts, per component. Pinning the current worst case keeps the headroom under
/// [`MAX_TAIL_COMBINATIONS`] visible: a new config air multiplies it, it does not add to it.
#[test]
fn the_sweep_stays_within_its_ceiling() {
    // Worst case: every operation has a tail, so every one contributes its full candidate count.
    // The little-endian and the big-endian ops never share an air, so they are two components and
    // the bound applies to each one.
    let airs = all_air_ids();
    let metas = air_metas();
    for ops in [ArithEqOp::ALL_LE, ArithEqOp::ALL_BE] {
        let combinations: u64 = ops
            .iter()
            .map(|&op| {
                metas.iter().filter(|m| airs.contains(&m.air_id) && m.covers(op)).count() as u64
            })
            .product();

        // An operation's candidates are its config's heights plus the two universal airs: four for
        // the two arith256, the two secp256k1 and the five bn254 operations (two heights each), two
        // for the secp256r1 pair that only the universal airs prove.
        assert_eq!(combinations, 4u64.pow(2) * 4u64.pow(2) * 4u64.pow(5) * 2u64.pow(2));
        assert!(
            combinations <= MAX_TAIL_COMBINATIONS,
            "{combinations} placements exceed the {MAX_TAIL_COMBINATIONS} the sweep is sized for",
        );
    }
}

/// Every little-endian and every big-endian operation with a tail at once: swept jointly that would
/// be the square of the per-endianness worst case, far past the ceiling. Placed per component it is
/// two sweeps, and each op lands only in airs of its own endianness.
#[test]
fn little_and_big_endian_tails_are_placed_independently() {
    let cap = cap(&meta_of(ArithEqTrace::<()>::AIR_ID));
    let pairs: Vec<(ArithEqOp, u64)> =
        ArithEqOp::ALL.iter().enumerate().map(|(i, &op)| (op, cap + 1 + i as u64)).collect();
    let totals = counts(&pairs);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);

    let be_airs = big_endian_air_ids();
    for p in &plan {
        for (idx, &n) in p.op_counts.iter().enumerate() {
            if n == 0 {
                continue;
            }
            let op = ArithEqOp::ALL[idx];
            assert_eq!(
                be_airs.contains(&p.air_id),
                op.is_big_endian(),
                "{op:?} planned in air {} of the other endianness",
                p.air_id
            );
        }
    }

    // The placement of each half is the one the half would get on its own.
    for ops in [&ArithEqOp::ALL_LE[..], &ArithEqOp::ALL_BE[..]] {
        let half: Vec<(ArithEqOp, u64)> =
            pairs.iter().copied().filter(|(op, _)| ops.contains(op)).collect();
        let half_totals = counts(&half);
        let half_plan = plan_air_strategy(&all_air_ids(), &half_totals);
        let expected: Vec<&ArithEqAirPlan> =
            plan.iter().filter(|p| ops.iter().any(|op| p.op_counts[op.index()] > 0)).collect();
        assert_eq!(half_plan.iter().collect::<Vec<_>>(), expected);
    }
}

/// A big-endian operation is proved by the big-endian airs only: the little-endian ones, however
/// cheap, are never candidates. Two small big-endian tails pool into the one universal big-endian
/// instance the secp256r1 pair needs anyway, as their little-endian twins would.
#[test]
fn big_endian_ops_take_the_big_endian_airs() {
    let totals = counts(&[(ArithEqOp::Secp256k1AddBe, 3), (ArithEqOp::Secp256r1DblBe, 2)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);
    assert_eq!(plan.len(), 1, "{plan:?}");
    assert_eq!(plan[0].air_id, ArithEqBeTrace::<()>::AIR_ID, "{plan:?}");

    // A bulk of a specialized big-endian op goes to its own big-endian config.
    let cap = cap(&meta_of(ArithSecp256K1BeLargeTrace::<()>::AIR_ID));
    let totals = counts(&[(ArithEqOp::Secp256k1AddBe, cap)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);
    assert_eq!(plan.len(), 1, "{plan:?}");
    assert_eq!(plan[0].air_id, ArithSecp256K1BeLargeTrace::<()>::AIR_ID, "{plan:?}");
}

/// A handful of operations must not open a tall instance: one instance either way, so the memory
/// tie-break sends them to the narrowest, shortest air that covers them.
#[test]
fn a_small_family_takes_the_cheapest_air_that_covers_it() {
    let totals = counts(&[(ArithEqOp::Arith256, 3)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, Arith256XTrace::<()>::AIR_ID);
    assert_eq!(plan[0].instances, 1);
}

#[test]
fn one_family_two_ops_uses_single_covering_air() {
    let totals = counts(&[(ArithEqOp::Arith256, 2), (ArithEqOp::Arith256Mod, 1)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, Arith256XTrace::<()>::AIR_ID);
}

/// The criterion's headline: leftovers of unrelated families pool into one instance rather than
/// taking one each, because the instance count is what it looks at first.
#[test]
fn small_leftovers_consolidate_into_one_instance() {
    let totals = counts(&[(ArithEqOp::Arith256Mod, 4), (ArithEqOp::Secp256k1Add, 5)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);

    assert_eq!(plan_instances(&plan), 1, "one instance holds both leftovers");
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, ArithEqTrace::<()>::AIR_ID, "and it is the short universal air");
}

/// Work that fills whole instances goes to the air that needs the fewest of them, ties broken by the
/// least memory. `Arith256XLarge` and the universal `ArithEqLarge` are the same height, so they tie
/// on instance count and the narrower specialized air wins on memory.
#[test]
fn a_bulk_goes_where_the_fewest_instances_are_needed() {
    let tallest = meta_of(Arith256XLargeTrace::<()>::AIR_ID);
    let cap_tall = cap(&tallest);
    let totals = counts(&[(ArithEqOp::Arith256, 3 * cap_tall)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, Arith256XLargeTrace::<()>::AIR_ID);
    assert_eq!(plan[0].instances, 3);

    // The step below in its own ladder is half the height, so it would need twice the instances —
    // which the criterion rules out before memory is ever compared.
    let below = meta_of(Arith256XTrace::<()>::AIR_ID);
    assert!((3 * cap_tall).div_ceil(cap(&below)) > 3);

    // The universal tall air is the same height and so ties on instance count; being wider it holds
    // the same work in more memory, which is what sends the bulk to the specialized air.
    let universal = meta_of(ArithEqLargeTrace::<()>::AIR_ID);
    assert_eq!(cap(&universal), cap_tall);
    assert!(memory(&universal, 3 * cap_tall) > plan_area(&plan));
}

/// A bulk's tail can land away from the bulk, splitting one operation across two airs — here into
/// the instance another family needs anyway, which costs no extra instance at all.
#[test]
fn a_tail_rides_in_an_instance_another_family_needs() {
    let cap_tall = cap(&meta_of(Arith256XLargeTrace::<()>::AIR_ID));
    let totals = counts(&[(ArithEqOp::Arith256, 3 * cap_tall + 10), (ArithEqOp::Secp256r1Add, 1)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);

    let bulk = plan.iter().find(|p| p.air_id == Arith256XLargeTrace::<()>::AIR_ID).unwrap();
    assert_eq!(bulk.op_counts[ArithEqOp::Arith256.index()], 3 * cap_tall);
    assert_eq!(bulk.instances, 3);

    // secp256r1 is only provable by a universal air, and the arith256 tail joins it there rather
    // than opening a fifth instance.
    assert_eq!(plan_instances(&plan), 4);
    let pooled = plan.iter().find(|p| p.air_id == ArithEqTrace::<()>::AIR_ID).unwrap();
    assert_eq!(pooled.op_counts[ArithEqOp::Arith256.index()], 10);
    assert_eq!(pooled.op_counts[ArithEqOp::Secp256r1Add.index()], 1);
    assert_eq!(pooled.instances, 1);
}

#[test]
fn ops_without_a_specialized_air_go_to_the_universal_one() {
    let totals = counts(&[(ArithEqOp::Secp256r1Add, 3), (ArithEqOp::Secp256r1Dbl, 2)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);

    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, ArithEqTrace::<()>::AIR_ID);
}

#[test]
fn absent_airs_are_never_planned() {
    // Without the arith256 airs in the pilout, those operations have only the universal ones left.
    let totals = counts(&[(ArithEqOp::Arith256, 2), (ArithEqOp::Arith256Mod, 1)]);
    let present = vec![ArithEqTrace::<()>::AIR_ID, ArithSecp256K1Trace::<()>::AIR_ID];
    let plan = plan_air_strategy(&present, &totals);
    assert_conserves(&plan, &totals);

    assert!(plan.iter().all(|p| present.contains(&p.air_id)));
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, ArithEqTrace::<()>::AIR_ID);
}

/// Tails are placed **whole**: the sweep chooses which air takes a leftover, never how to divide it
/// between several. So an operation ends up in at most two airs — the one holding its bulk and the
/// one holding its tail — never spread further. This is the module's *Known gap* stated as the
/// property it actually guarantees.
#[test]
fn an_operation_is_never_spread_beyond_its_bulk_and_its_tail() {
    let c = cap(&meta_of(Arith256XTrace::<()>::AIR_ID));
    let totals = counts(&[
        (ArithEqOp::Arith256, 5 * c + c / 2),
        (ArithEqOp::Arith256Mod, 3 * c / 4),
        (ArithEqOp::Secp256k1Add, 3 * c / 4),
        (ArithEqOp::Secp256r1Add, 3 * c / 4),
    ]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);

    for op in ArithEqOp::ALL {
        let airs = plan.iter().filter(|p| p.op_counts[op.index()] > 0).count();
        assert!(airs <= 2, "{op:?} was spread over {airs} airs, more than a bulk and a tail");
    }
}

#[test]
#[should_panic(expected = "covered by no present air")]
fn an_op_no_present_air_covers_panics() {
    let totals = counts(&[(ArithEqOp::Secp256r1Add, 1)]);
    plan_air_strategy(&[Arith256XTrace::<()>::AIR_ID], &totals);
}
