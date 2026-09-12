//! Unit tests for the ArithEq air-selection planner (moved out of `arith_eq_planner.rs`).
//! Declared there via `#[cfg(test)] #[path = "tests/arith_eq_planner_tests.rs"] mod tests;`,
//! so it stays a child module of `arith_eq_planner` and keeps `super::` access to privates.

use super::*;
use zisk_pil::{
    Arith256XLargeTrace, Arith256XTrace, ArithBn254LargeTrace, ArithBn254Trace, ArithEqHugeTrace,
    ArithEqLargeTrace, ArithEqTrace, ArithSecp256K1LargeTrace, ArithSecp256K1Trace,
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
        ArithEqHugeTrace::<()>::AIR_ID,
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

/// Every config must be a size ladder: each step strictly taller than the one below, exactly as
/// wide, and covering the same operations. That is what the strategy relies on to trade memory for a
/// lower instance count. The ladders are walked pairwise rather than assumed to be pairs, so a
/// config that gains a third height in `zisk.pil` is still checked step by step.
#[test]
fn every_config_is_a_size_ladder() {
    for ladder in [
        &[
            ArithEqTrace::<()>::AIR_ID,
            ArithEqLargeTrace::<()>::AIR_ID,
            ArithEqHugeTrace::<()>::AIR_ID,
        ][..],
        &[Arith256XTrace::<()>::AIR_ID, Arith256XLargeTrace::<()>::AIR_ID][..],
        &[ArithSecp256K1Trace::<()>::AIR_ID, ArithSecp256K1LargeTrace::<()>::AIR_ID][..],
        &[ArithBn254Trace::<()>::AIR_ID, ArithBn254LargeTrace::<()>::AIR_ID][..],
    ] {
        for step in ladder.windows(2) {
            let (short, tall) = (meta_of(step[0]), meta_of(step[1]));
            assert!(tall.num_rows > short.num_rows, "air {} must be taller", tall.air_id);
            assert!(tall.cost > short.cost, "air {} must cost more, being taller", tall.air_id);
            assert_eq!(tall.ops, short.ops, "air {} must cover the same operations", tall.air_id);
        }
    }
}

/// The sweep is exhaustive over which air takes each tail, so the size of the space it searches is
/// the product of the candidate counts. Pinning the current worst case keeps the headroom under
/// [`MAX_TAIL_COMBINATIONS`] visible: a new air multiplies it, it does not add to it. What it
/// actually *visits* is far less — see `the_worst_case_sweep_stays_fast` for the bound that cuts it.
#[test]
fn the_sweep_stays_within_its_ceiling() {
    // Worst case: every operation has a tail, so every one contributes its full candidate count.
    let airs = all_air_ids();
    let metas = air_metas();
    let combinations: u64 = ArithEqOp::ALL
        .iter()
        .map(|&op| metas.iter().filter(|m| airs.contains(&m.air_id) && m.covers(op)).count() as u64)
        .product();

    // An operation's candidates are its config's heights plus the three universal airs: five for
    // the two arith256, the two secp256k1 and the five bn254 operations (two heights each), three
    // for the secp256r1 pair that only the universal airs prove.
    assert_eq!(combinations, 5u64.pow(2) * 5u64.pow(2) * 5u64.pow(5) * 3u64.pow(2));
    assert!(
        combinations <= MAX_TAIL_COMBINATIONS,
        "{combinations} placements exceed the {MAX_TAIL_COMBINATIONS} the sweep is sized for",
    );
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
/// least memory. `Arith256XLarge` and the universal `ArithEqHuge` are the same height — the top of
/// both ladders — so they tie on instance count and the narrower specialized air wins on memory.
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

    // The tallest universal air is the same height and so ties on instance count; being wider it
    // holds the same work in more memory, which is what sends the bulk to the specialized air.
    let universal = meta_of(ArithEqHugeTrace::<()>::AIR_ID);
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

/// Reference sweep: the flat, unpruned mixed-radix enumeration the planner used before the
/// depth-first one, kept here so the pruned search can be checked against it. It rebuilds the bulk
/// and the tails exactly as [`plan_air_strategy`] does — the part under test is only *how* the tail
/// placements are searched, so everything above them has to be the same to compare like with like.
fn plan_by_flat_sweep(
    present_air_ids: &[usize],
    op_counts: &[u64; ARITH_EQ_OP_NUM],
) -> Vec<ArithEqAirPlan> {
    let metas: Vec<ArithEqAirMeta> =
        air_metas().into_iter().filter(|m| present_air_ids.contains(&m.air_id)).collect();
    let caps: Vec<u64> = metas.iter().map(cap).collect();

    let mut bulk_rows = vec![0u64; metas.len()];
    let mut air_counts = vec![[0u64; ARITH_EQ_OP_NUM]; metas.len()];
    let mut tails: Vec<(usize, u64, Vec<usize>)> = Vec::new();

    for (op_idx, &count) in op_counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let op = ArithEqOp::ALL[op_idx];
        let candidates: Vec<usize> = (0..metas.len()).filter(|&j| metas[j].covers(op)).collect();
        let bulk_air = *candidates
            .iter()
            .min_by_key(|&&j| (std::cmp::Reverse(caps[j]), metas[j].cost))
            .unwrap();
        let bulk = count / caps[bulk_air] * caps[bulk_air];
        if bulk > 0 {
            bulk_rows[bulk_air] += bulk;
            air_counts[bulk_air][op_idx] += bulk;
        }
        if count > bulk {
            tails.push((op_idx, count - bulk, candidates));
        }
    }

    let mut choice = vec![0usize; tails.len()];
    let mut best_choice = choice.clone();
    let mut best = Cost { instances: u64::MAX, memory: u64::MAX };
    loop {
        let mut rows = bulk_rows.clone();
        for ((_, n, cands), &c) in tails.iter().zip(choice.iter()) {
            rows[cands[c]] += n;
        }
        let mut total = Cost::default();
        for (j, &r) in rows.iter().enumerate() {
            if r != 0 {
                let instances = r.div_ceil(caps[j]);
                total.instances += instances;
                total.memory += instances * metas[j].cost as u64;
            }
        }
        if total < best {
            best = total;
            best_choice.copy_from_slice(&choice);
        }
        let mut carry = true;
        for (pos, digit) in choice.iter_mut().enumerate().rev() {
            *digit += 1;
            if *digit < tails[pos].2.len() {
                carry = false;
                break;
            }
            *digit = 0;
        }
        if carry {
            break;
        }
    }

    for ((op_idx, n, cands), &c) in tails.iter().zip(best_choice.iter()) {
        air_counts[cands[c]][*op_idx] += n;
    }
    metas
        .iter()
        .zip(air_counts)
        .filter_map(|(m, counts)| {
            let total: u64 = counts.iter().sum();
            (total > 0).then(|| ArithEqAirPlan {
                air_id: m.air_id,
                op_counts: counts,
                instances: total.div_ceil(cap(m)),
            })
        })
        .collect()
}

/// The bound the depth-first sweep prunes with only ever cuts subtrees that cannot hold a strictly
/// better placement, so it must return *the same plan* as the exhaustive flat sweep — not merely one
/// of equal cost, since the tie-break ("first placement in order wins") is what decides between
/// equal-cost plans and callers rely on it being stable.
///
/// The reference is exponential in the number of operations that have a tail, so the cases are
/// capped at [`REFERENCE_BUDGET`] placements each: the shapes that matter for the search are how the
/// tails fall relative to the capacities, not how many operations are live at once, and the
/// all-eleven case is covered by `the_worst_case_sweep_stays_fast` and by the filler tests.
#[test]
fn the_pruned_sweep_agrees_with_the_exhaustive_one() {
    /// Placements a single case may ask the flat reference to enumerate.
    const REFERENCE_BUDGET: u64 = 100_000;

    let airs = all_air_ids();
    let metas = air_metas();
    // Deterministic walk over op-count vectors: a fixed xorshift, so a failure is reproducible. The
    // magnitudes straddle the capacities — some operations only have a tail, some fill whole
    // instances — which is what makes the bulk/tail split, and therefore the search, non-trivial.
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let max_cap = metas.iter().map(cap).max().unwrap();

    let mut compared = 0usize;
    for case in 0..600 {
        let live = 1 + (next() % 5) as usize; // 1..=5 operations carry work
        let mut totals = [0u64; ARITH_EQ_OP_NUM];
        for _ in 0..live {
            let op_idx = (next() % ARITH_EQ_OP_NUM as u64) as usize;
            let r = next();
            // Half the operations are a bare tail, half a bulk plus a tail.
            totals[op_idx] = if r % 2 == 0 { 1 + r % 64 } else { 1 + r % (max_cap * 3) };
        }

        let placements: u64 = totals
            .iter()
            .enumerate()
            .filter(|(_, &c)| c != 0)
            .map(|(i, _)| metas.iter().filter(|m| m.covers(ArithEqOp::ALL[i])).count() as u64)
            .product();
        if placements > REFERENCE_BUDGET {
            continue;
        }

        let got = plan_air_strategy(&airs, &totals);
        let want = plan_by_flat_sweep(&airs, &totals);
        assert_eq!(got, want, "case {case}: totals {totals:?}");
        compared += 1;
    }
    assert!(
        compared > 200,
        "only {compared} cases fit the reference budget; the walk is too narrow"
    );
}

/// The all-operations worst case is what the pruning exists for: unpruned it enumerates
/// `MAX_TAIL_COMBINATIONS`-worth of placements and measured 323 ms in release, which a planner that
/// runs once per block cannot afford. The bound is generous — the point is to catch a regression
/// that removes the prune, not to pin a benchmark.
#[test]
fn the_worst_case_sweep_stays_fast() {
    let airs = all_air_ids();
    let totals = counts(
        &ArithEqOp::ALL.iter().enumerate().map(|(i, &op)| (op, i as u64 + 1)).collect::<Vec<_>>(),
    );
    let started = std::time::Instant::now();
    let plan = plan_air_strategy(&airs, &totals);
    let elapsed = started.elapsed();
    assert_conserves(&plan, &totals);
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "the worst-case sweep took {elapsed:?}; the pruning bound is not cutting subtrees",
    );
}
