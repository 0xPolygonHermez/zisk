//! Unit tests for the ArithEq air-selection planner (moved out of `arith_eq_planner.rs`).
//! Declared there via `#[cfg(test)] #[path = "tests/arith_eq_planner_tests.rs"] mod tests;`,
//! so it stays a child module of `arith_eq_planner` and keeps `super::` access to privates.

use super::*;
use zisk_pil::{
    Arith256XLargeTrace, Arith256XTrace, ArithBn254LargeTrace, ArithBn254Trace, ArithEqLargeTrace,
    ArithEqTrace, ArithSecp256K1LargeTrace, ArithSecp256K1Trace,
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
    plan.iter()
        .map(|p| {
            let m = meta_of(p.air_id);
            p.instances * m.num_rows as u64 * m.row_size as u64
        })
        .sum()
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
    ]
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

/// The air table must list every config at two heights, the tall one strictly taller and exactly as
/// wide. That is what the strategy relies on to trade area for a lower instance count.
#[test]
fn every_config_is_a_size_ladder() {
    for (short, tall) in [
        (ArithEqTrace::<()>::AIR_ID, ArithEqLargeTrace::<()>::AIR_ID),
        (Arith256XTrace::<()>::AIR_ID, Arith256XLargeTrace::<()>::AIR_ID),
        (ArithSecp256K1Trace::<()>::AIR_ID, ArithSecp256K1LargeTrace::<()>::AIR_ID),
        (ArithBn254Trace::<()>::AIR_ID, ArithBn254LargeTrace::<()>::AIR_ID),
    ] {
        let (short, tall) = (meta_of(short), meta_of(tall));
        assert!(tall.num_rows > short.num_rows, "air {} must be taller", tall.air_id);
        assert_eq!(tall.row_size, short.row_size, "air {} must be as wide", tall.air_id);
        assert_eq!(tall.ops, short.ops, "air {} must cover the same operations", tall.air_id);
    }
}

/// The sweep is exhaustive over which air takes each tail, so its cost is the product of the
/// candidate counts. Pinning the current worst case keeps the headroom under
/// [`MAX_TAIL_COMBINATIONS`] visible: a new config air multiplies it, it does not add to it.
#[test]
fn the_sweep_stays_within_its_ceiling() {
    // Worst case: every operation has a tail, so every one contributes its full candidate count.
    let airs = all_air_ids();
    let metas = air_metas();
    let combinations: u64 = ArithEqOp::ALL
        .iter()
        .map(|&op| metas.iter().filter(|m| airs.contains(&m.air_id) && m.covers(op)).count() as u64)
        .product();

    // Four candidates for an operation a specialised config covers (that config's two heights plus
    // the two universal airs), two for the secp256r1 pair that only the universal airs prove.
    assert_eq!(combinations, 4u64.pow(9) * 2u64.pow(2));
    assert!(
        combinations <= MAX_TAIL_COMBINATIONS,
        "{combinations} placements exceed the {MAX_TAIL_COMBINATIONS} the sweep is sized for",
    );
}

/// A handful of operations must not open a tall instance: one instance either way, so the area
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

/// Work that fills whole instances goes to the air that needs the fewest of them, which is the tall
/// universal one — even though a specialized air would take less area per operation.
#[test]
fn a_bulk_goes_where_the_fewest_instances_are_needed() {
    let cap_tall = cap(&meta_of(ArithEqLargeTrace::<()>::AIR_ID));
    let totals = counts(&[(ArithEqOp::Arith256, 3 * cap_tall)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, ArithEqLargeTrace::<()>::AIR_ID);
    assert_eq!(plan[0].instances, 3);

    // The specialized air is narrower but shorter, so it would need more instances — which the
    // criterion rules out before area is ever compared.
    let narrow = meta_of(Arith256XLargeTrace::<()>::AIR_ID);
    assert!((3 * cap_tall).div_ceil(cap(&narrow)) > 3);
    assert!(plan_area(&plan) > area(&narrow, 3 * cap_tall), "and it really is dearer in area");
}

/// A bulk's tail can land away from the bulk, splitting one operation across two airs — here into
/// the instance another family needs anyway, which costs no extra instance at all.
#[test]
fn a_tail_rides_in_an_instance_another_family_needs() {
    let cap_tall = cap(&meta_of(ArithEqLargeTrace::<()>::AIR_ID));
    let totals = counts(&[(ArithEqOp::Arith256, 3 * cap_tall + 10), (ArithEqOp::Secp256r1Add, 1)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);

    let bulk = plan.iter().find(|p| p.air_id == ArithEqLargeTrace::<()>::AIR_ID).unwrap();
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
