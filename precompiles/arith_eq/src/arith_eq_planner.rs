//! Area-minimizing air-selection strategy for the `ArithEq` family.
//!
//! The counter tallies each sub-operation separately (`[u64; ARITH_EQ_OP_NUM]`). This module turns
//! those totals — together with the airs actually present in the pilout — into a per-air, per-op
//! assignment that covers every observed operation at (heuristically) minimal total area.
//!
//! Area model: every instance is a full `num_rows` trace regardless of how full it is, so its area is
//! `instances · num_rows · row_size`. A specialized air (fewer columns → smaller `row_size`) is the
//! cheapest way to prove a *full* instance of its operations; but a partially-filled specialized
//! instance can cost more area than folding its leftover into a broader air shared with other
//! leftovers.
//!
//! Strategy (per family = PIL equation group):
//!   * Its `full = count / cap` **full instances always go to its specialized air** (cheapest per-op).
//!   * Its `remainder = count % cap` is either kept in one extra partial specialized instance, or
//!     **pooled into the universal (full `ArithEq`) air** together with other families' leftovers.
//!
//! We enumerate which remainders to pool and pick the least total area. Because a remainder can be
//! pooled, a single operation type may be **split** across its specialized air (its full instances)
//! and the universal air (its remainder) — the per-air `op_counts` capture that split, and the
//! planner's filler assigns non-overlapping per-op collect windows (specialized ranks first).

use crate::{air_metas, ArithEqAirMeta, ArithEqOp, ARITH_EQ_OP_NUM, ARITH_EQ_ROWS_BY_OP};

/// Intrinsic `ArithEq` operation families (PIL equation groups): ops within a family are proved by
/// the same specialized air.
const FAMILIES: &[&[ArithEqOp]] = &[
    &[ArithEqOp::Arith256, ArithEqOp::Arith256Mod],
    &[ArithEqOp::Secp256k1Add, ArithEqOp::Secp256k1Dbl],
    &[ArithEqOp::Bn254CurveAdd, ArithEqOp::Bn254CurveDbl],
    &[ArithEqOp::Bn254ComplexAdd, ArithEqOp::Bn254ComplexSub, ArithEqOp::Bn254ComplexMul],
    &[ArithEqOp::Secp256r1Add, ArithEqOp::Secp256r1Dbl],
];

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

#[inline]
fn cap(m: &ArithEqAirMeta) -> u64 {
    m.num_rows as u64 / ARITH_EQ_ROWS_BY_OP as u64
}

/// Total area to prove `ops` operations in air `m`: `ceil(ops/cap) · num_rows · row_size`.
#[inline]
fn area(m: &ArithEqAirMeta, ops: u64) -> u64 {
    ops.div_ceil(cap(m)) * m.num_rows as u64 * m.row_size as u64
}

/// Compute the per-air, per-op assignment for the given per-op totals, considering only
/// `present_air_ids` (the airs in the pilout), minimizing total instance area. The returned plans put
/// the universal (full) air last, so the filler assigns specialized ranks of each op first. Panics if
/// an observed operation is covered by no present air.
pub fn plan_air_strategy(
    present_air_ids: &[usize],
    op_counts: &[u64; ARITH_EQ_OP_NUM],
) -> Vec<ArithEqAirPlan> {
    let metas: Vec<ArithEqAirMeta> =
        air_metas().into_iter().filter(|m| present_air_ids.contains(&m.air_id)).collect();
    if metas.is_empty() {
        return Vec::new();
    }

    // Universal air = one that covers every operation (the full `ArithEq`); pools leftovers.
    let universal = metas.iter().find(|m| ArithEqOp::ALL.iter().all(|&op| m.covers(op))).copied();
    let universal_id = universal.as_ref().map(|u| u.air_id);

    // Present families: total count, full/remainder split, cheapest covering specialized air.
    struct Family {
        ops: Vec<usize>,
        count: u64,
        spec: Option<ArithEqAirMeta>,
        full: u64,
        rem: u64,
    }
    let mut families: Vec<Family> = Vec::new();
    for fam in FAMILIES {
        let ops: Vec<usize> =
            fam.iter().map(|op| op.index()).filter(|&i| op_counts[i] > 0).collect();
        if ops.is_empty() {
            continue;
        }
        let count: u64 = ops.iter().map(|&i| op_counts[i]).sum();
        let spec = metas
            .iter()
            .filter(|m| Some(m.air_id) != universal_id)
            .filter(|m| ops.iter().all(|&i| m.covers(ArithEqOp::ALL[i])))
            .min_by_key(|m| m.row_size)
            .copied();
        let (full, rem) = match &spec {
            Some(s) => (count / cap(s), count % cap(s)),
            None => (0, count), // no specialized air → everything pools into the universal air
        };
        families.push(Family { ops, count, spec, full, rem });
    }
    if families.is_empty() {
        return Vec::new();
    }

    // Full instances always go specialized (fixed area). Only remainders are decided.
    let full_area: u64 = families
        .iter()
        .filter_map(|f| f.spec.as_ref().map(|s| f.full * s.num_rows as u64 * s.row_size as u64))
        .sum();
    let forced_pool: u64 = families.iter().filter(|f| f.spec.is_none()).map(|f| f.count).sum();

    // Candidates for the pool-vs-keep decision: families with a specialized air and a remainder.
    let candidates: Vec<usize> = (0..families.len())
        .filter(|&i| families[i].spec.is_some() && families[i].rem > 0)
        .collect();

    let mut best_mask = 0u64;
    let mut best_area = u64::MAX;
    for mask in 0..(1u64 << candidates.len()) {
        let mut spec_partial = 0u64;
        let mut pool = forced_pool;
        for (bit, &fi) in candidates.iter().enumerate() {
            let s = families[fi].spec.as_ref().unwrap();
            if mask & (1 << bit) != 0 {
                pool += families[fi].rem; // pool this remainder
            } else {
                spec_partial += s.num_rows as u64 * s.row_size as u64; // one extra partial instance
            }
        }
        let pool_area = if pool > 0 {
            match &universal {
                Some(u) => area(u, pool),
                None => continue, // cannot pool without a universal air
            }
        } else {
            0
        };
        let total = full_area + spec_partial + pool_area;
        if total < best_area {
            best_area = total;
            best_mask = mask;
        }
    }
    assert!(
        best_area != u64::MAX,
        "plan_air_strategy: some operation is covered by no present air"
    );

    // Materialize per-air, per-op counts. `pool_promoted[i]` = remainder of candidate i goes to univ.
    let promoted: std::collections::HashSet<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(bit, _)| best_mask & (1 << bit) != 0)
        .map(|(_, &fi)| fi)
        .collect();

    let mut spec_counts: Vec<(usize, [u64; ARITH_EQ_OP_NUM])> = Vec::new(); // (air_id, counts)
    let mut univ_counts = [0u64; ARITH_EQ_OP_NUM];

    for (i, f) in families.iter().enumerate() {
        match &f.spec {
            Some(s) => {
                // Ops kept in the specialized air: all of them unless this family's remainder is
                // pooled, in which case only the `full · cap` that fill whole instances stay.
                let spec_total = if promoted.contains(&i) { f.full * cap(s) } else { f.count };
                let mut remaining = spec_total;
                let entry = spec_counts.iter_mut().find(|(id, _)| *id == s.air_id);
                let counts = match entry {
                    Some((_, c)) => c,
                    None => {
                        spec_counts.push((s.air_id, [0u64; ARITH_EQ_OP_NUM]));
                        &mut spec_counts.last_mut().unwrap().1
                    }
                };
                for &op_idx in &f.ops {
                    let to_spec = op_counts[op_idx].min(remaining);
                    counts[op_idx] += to_spec;
                    remaining -= to_spec;
                    univ_counts[op_idx] += op_counts[op_idx] - to_spec;
                }
            }
            None => {
                for &op_idx in &f.ops {
                    univ_counts[op_idx] += op_counts[op_idx];
                }
            }
        }
    }

    // Emit specialized plans first, then the universal pool (so the filler assigns specialized ranks
    // of each split op before the universal ranks).
    let mut plans: Vec<ArithEqAirPlan> = Vec::new();
    for (air_id, counts) in spec_counts {
        let m = metas.iter().find(|m| m.air_id == air_id).unwrap();
        let total: u64 = counts.iter().sum();
        if total == 0 {
            continue;
        }
        plans.push(ArithEqAirPlan { air_id, op_counts: counts, instances: total.div_ceil(cap(m)) });
    }
    let univ_total: u64 = univ_counts.iter().sum();
    if univ_total > 0 {
        let u = universal.as_ref().expect("universal air required to pool leftovers");
        plans.push(ArithEqAirPlan {
            air_id: u.air_id,
            op_counts: univ_counts,
            instances: univ_total.div_ceil(cap(u)),
        });
    }
    plans
}

#[cfg(test)]
#[path = "tests/arith_eq_planner_tests.rs"]
mod tests;
