//! Area-minimizing air-selection strategy for the `ArithEq` family.
//!
//! The counter tallies each sub-operation separately (`[u64; ARITH_EQ_OP_NUM]`). This module turns
//! those totals — together with the airs actually present in the pilout — into a per-air, per-op
//! assignment that covers every observed operation, cheaply in total area.
//!
//! Area model: every instance is a full `num_rows` trace regardless of how full it is, so its area is
//! `instances · num_rows · row_size`. A specialized air (fewer columns → smaller `row_size`) is the
//! cheapest way to prove a *full* instance of its operations; but a partially-filled specialized
//! instance can cost more area than folding its leftover into a broader air shared with other
//! leftovers.
//!
//! Strategy, per operation (not per PIL equation group: an air may cover only part of a group, and
//! `Arith256` is proved by both the 11-column `Arith256` air and the 19-column `Arith256X` one):
//!   * Its `bulk = ⌊count/cap⌋·cap` — the part that fills whole instances — **always goes to the
//!     covering air with the smallest row**. Since `cap = num_rows / ARITH_EQ_ROWS_BY_OP`, a filled
//!     instance costs `ARITH_EQ_ROWS_BY_OP · row_size` per operation whatever `num_rows` is, so no
//!     other air can prove those instances for less. And a bulk is a whole number of instances, so
//!     it can never absorb anyone's leftover: the choice is independent of everything below.
//!   * Its `tail = count % cap` may go to **any** air that covers it — its own cheapest air (i.e.
//!     one extra partial instance), another specialized air whose partial instance it can share, or
//!     the universal air pooling every leftover.
//!
//! Only the tails are searched, and each one is placed **whole**: the sweep is exhaustive over which
//! air takes a tail, not over how a tail might be divided between several. Because a tail can land
//! away from its bulk, a single operation may still be **split** across two airs — the per-air
//! `op_counts` capture that split, and the planner's filler assigns non-overlapping per-op collect
//! windows in the order the plans are returned.
//!
//! # Known gap
//!
//! Placing tails whole is *not* globally area-minimal. Dividing one tail to top up the spare
//! capacity of two other airs can empty an air completely, which the sweep cannot see: with equal
//! capacities `c` and counts `Arith256 = c/2`, `Arith256Mod = 3c/4`, `Secp256r1Add = 3c/4` it picks
//! one instance each of `Arith256`, `Arith256X` and `ArithEq`, where splitting the `Arith256` tail
//! into `c/4 + c/4` fills `Arith256X` and `ArithEq` exactly and drops the `Arith256` instance — 14.7%
//! less area. `indivisible_tails_are_a_known_gap` pins this down. Searching divisible placements is
//! bin packing with splitting, so it needs a different algorithm, not a wider sweep.

use crate::{air_metas, ArithEqAirMeta, ArithEqOp, ARITH_EQ_OP_NUM, ARITH_EQ_ROWS_BY_OP};

/// Upper bound on the tail placements this exhaustive search will enumerate. The current air table
/// yields `3 · 2^8 = 768`; the bound is here so that adding heavily overlapping airs fails loudly
/// instead of silently hanging, since optimal tail placement is a bin-packing problem.
const MAX_TAIL_COMBINATIONS: u64 = 1 << 20;

/// One planned air: how many of each operation it proves. Ops with a non-zero count feed this air;
/// the same op may also appear (with the complementary count) in another air when split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArithEqAirPlan {
    pub air_id: usize,
    pub op_counts: [u64; ARITH_EQ_OP_NUM],
    /// Instances needed: `ceil(sum(op_counts) / cap)`.
    pub instances: u64,
}

impl ArithEqAirPlan {
    pub fn total_ops(&self) -> u64 {
        self.op_counts.iter().sum()
    }
}

/// Operations one instance of `m` can prove.
#[inline]
fn cap(m: &ArithEqAirMeta) -> u64 {
    m.num_rows as u64 / ARITH_EQ_ROWS_BY_OP as u64
}

/// Area of one instance of `m`, full or not.
#[inline]
fn instance_area(m: &ArithEqAirMeta) -> u64 {
    m.num_rows as u64 * m.row_size as u64
}

/// Total area to prove `ops` operations in air `m`: `ceil(ops/cap) · num_rows · row_size`.
///
/// The definition of the cost model, kept for the tests to state expected areas with. The sweep in
/// `plan_air_strategy` inlines it against hoisted `caps`/`instance_areas` instead of calling it, so
/// it does not redo both divisions once per air per combination.
#[cfg(test)]
#[inline]
fn area(m: &ArithEqAirMeta, ops: u64) -> u64 {
    ops.div_ceil(cap(m)) * instance_area(m)
}

/// A leftover to place: fewer than `cap` operations of one op, and the airs that could take them.
struct Tail {
    op_idx: usize,
    rows: u64,
    /// Indices into the `metas` slice.
    candidates: Vec<usize>,
}

/// Compute the per-air, per-op assignment for the given per-op totals, considering only
/// `present_air_ids` (the airs in the pilout), at the least total instance area among the placements
/// it searches — every tail placed whole, see the module's *Known gap*. Plans come back in
/// `air_metas()` order — cheapest/most-specific first, universal last — so a split op's specialized
/// collect windows are assigned before its universal ones. Panics if an observed operation is
/// covered by no present air.
pub fn plan_air_strategy(
    present_air_ids: &[usize],
    op_counts: &[u64; ARITH_EQ_OP_NUM],
) -> Vec<ArithEqAirPlan> {
    let metas: Vec<ArithEqAirMeta> =
        air_metas().into_iter().filter(|m| present_air_ids.contains(&m.air_id)).collect();

    // Rows every air receives no matter how the tails are placed, and which ops they came from.
    let mut bulk_rows = vec![0u64; metas.len()];
    let mut air_counts = vec![[0u64; ARITH_EQ_OP_NUM]; metas.len()];
    let mut tails: Vec<Tail> = Vec::new();

    for (op_idx, &count) in op_counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let op = ArithEqOp::ALL[op_idx];
        let candidates: Vec<usize> = (0..metas.len()).filter(|&j| metas[j].covers(op)).collect();
        assert!(!candidates.is_empty(), "plan_air_strategy: {op:?} is covered by no present air");

        let bulk_air = *candidates.iter().min_by_key(|&&j| metas[j].row_size).unwrap();
        let bulk = count / cap(&metas[bulk_air]) * cap(&metas[bulk_air]);
        if bulk > 0 {
            bulk_rows[bulk_air] += bulk;
            air_counts[bulk_air][op_idx] += bulk;
        }
        if count > bulk {
            tails.push(Tail { op_idx, rows: count - bulk, candidates });
        }
    }

    let combinations = tails
        .iter()
        .try_fold(1u64, |acc, t| acc.checked_mul(t.candidates.len() as u64))
        .unwrap_or(u64::MAX);
    assert!(
        combinations <= MAX_TAIL_COMBINATIONS,
        "plan_air_strategy: {combinations} tail placements exceed the {MAX_TAIL_COMBINATIONS} this \
         exhaustive search is sized for; the air table needs a smarter search"
    );

    // Hoisted out of the sweep below: `area` would otherwise recompute both divisions once per air
    // per combination.
    let caps: Vec<u64> = metas.iter().map(cap).collect();
    let instance_areas: Vec<u64> = metas.iter().map(instance_area).collect();

    // Mixed-radix sweep over the tail placements: choice[i] indexes tails[i].candidates.
    let mut choice = vec![0usize; tails.len()];
    let mut best_choice = choice.clone();
    let mut best_area = u64::MAX;
    let mut rows = vec![0u64; metas.len()];
    loop {
        rows.copy_from_slice(&bulk_rows);
        for (t, &c) in tails.iter().zip(choice.iter()) {
            rows[t.candidates[c]] += t.rows;
        }
        let total: u64 = rows
            .iter()
            .enumerate()
            .map(|(j, &r)| if r == 0 { 0 } else { r.div_ceil(caps[j]) * instance_areas[j] })
            .sum();
        if total < best_area {
            best_area = total;
            best_choice.copy_from_slice(&choice);
        }

        // Advance from the rightmost digit; a carry out of the leftmost means we are back to the
        // all-zeros placement and every combination has been seen. With no tails at all this exits
        // after the single evaluation above.
        let mut carry = true;
        for (pos, digit) in choice.iter_mut().enumerate().rev() {
            *digit += 1;
            if *digit < tails[pos].candidates.len() {
                carry = false;
                break;
            }
            *digit = 0;
        }
        if carry {
            break;
        }
    }

    for (t, &c) in tails.iter().zip(best_choice.iter()) {
        air_counts[t.candidates[c]][t.op_idx] += t.rows;
    }

    let plans: Vec<ArithEqAirPlan> = metas
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
        .collect();

    // Every counted operation must be planned exactly once: an op silently missing from every plan
    // would never be collected, and would only surface much later as an unbalanced bus.
    #[cfg(debug_assertions)]
    {
        let mut summed = [0u64; ARITH_EQ_OP_NUM];
        for plan in &plans {
            for (s, c) in summed.iter_mut().zip(plan.op_counts.iter()) {
                *s += *c;
            }
        }
        assert_eq!(&summed, op_counts, "plan_air_strategy dropped or duplicated operations");
    }

    plans
}

#[cfg(test)]
#[path = "tests/arith_eq_planner_tests.rs"]
mod tests;
