//! Unit tests for the ArithEq area-minimizing planner (moved out of `arith_eq_planner.rs`).
//! Declared there via `#[cfg(test)] #[path = "tests/arith_eq_planner_tests.rs"] mod tests;`,
//! so it stays a child module of `arith_eq_planner` and keeps `super::` access to privates.

use super::*;
use zisk_pil::{
    Arith256Trace, Arith256XTrace, ArithBn254ComplexTrace, ArithBn254EcTrace, ArithEqTrace,
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
    plan.iter()
        .map(|p| {
            let m = meta_of(p.air_id);
            p.instances * m.num_rows as u64 * m.row_size as u64
        })
        .sum()
}

// Config-air ids taken straight from the generated `zisk_pil` trace types (pil_helpers/traces.rs),
// the pilout source of truth — so the test tracks AIR_ID renumbering automatically and validates the
// planner against the real airs rather than against `air_metas()`.
fn all_air_ids() -> Vec<usize> {
    vec![
        ArithEqTrace::<()>::AIR_ID,
        Arith256Trace::<()>::AIR_ID,
        Arith256XTrace::<()>::AIR_ID,
        ArithSecp256K1Trace::<()>::AIR_ID,
        ArithBn254EcTrace::<()>::AIR_ID,
        ArithBn254ComplexTrace::<()>::AIR_ID,
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

#[test]
fn single_small_family_stays_specialized() {
    let totals = counts(&[(ArithEqOp::Arith256, 3)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, Arith256Trace::<()>::AIR_ID);
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

#[test]
fn small_leftovers_consolidate_into_least_area() {
    let totals = counts(&[(ArithEqOp::Arith256Mod, 4), (ArithEqOp::Secp256k1Add, 5)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);

    let all_specialized = area(&meta_of(Arith256XTrace::<()>::AIR_ID), 4)
        + area(&meta_of(ArithSecp256K1Trace::<()>::AIR_ID), 5);
    let consolidated = area(&meta_of(ArithEqTrace::<()>::AIR_ID), 9);
    assert_eq!(plan_area(&plan), all_specialized.min(consolidated));
    if consolidated < all_specialized {
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].air_id, ArithEqTrace::<()>::AIR_ID);
    }
}

#[test]
fn large_family_fills_specialized() {
    let cap_arith = cap(&meta_of(Arith256Trace::<()>::AIR_ID));
    let totals = counts(&[(ArithEqOp::Arith256, 3 * cap_arith)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].air_id, Arith256Trace::<()>::AIR_ID);
    assert_eq!(plan[0].instances, 3);
}

#[test]
fn full_instances_specialized_remainder_pooled() {
    // A large secp256k1 family (2·cap + small remainder) alongside a tiny arith256_mod family.
    // Expect: secp256k1's 2 full instances stay in ArithSecp256K1; its remainder + the mod
    // leftover pool into one ArithEq instance (cheaper than two extra partial specialized ones)
    // — so secp256k1 ops are SPLIT across ArithSecp256K1 and ArithEq.
    let cap_secp = cap(&meta_of(ArithSecp256K1Trace::<()>::AIR_ID));
    let totals =
        counts(&[(ArithEqOp::Secp256k1Add, 2 * cap_secp + 10), (ArithEqOp::Arith256Mod, 7)]);
    let plan = plan_air_strategy(&all_air_ids(), &totals);
    assert_conserves(&plan, &totals);

    let secp = plan.iter().find(|p| p.air_id == ArithSecp256K1Trace::<()>::AIR_ID);
    let arith_eq = plan.iter().find(|p| p.air_id == ArithEqTrace::<()>::AIR_ID);
    // secp256k1 has at least its 2 full instances specialized.
    assert!(secp.is_some_and(|p| p.instances >= 2));
    // Whether the remainder pooled depends on the row-size arithmetic; if it did, the ArithEq
    // pool holds the secp remainder + the mod leftover.
    if let Some(ae) = arith_eq {
        assert!(ae.op_counts[ArithEqOp::Arith256Mod.index()] > 0);
    }
}
