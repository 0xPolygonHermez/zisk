//! Hands the operations of a chunk to the airs that can prove them.
//!
//! Every binary operation belongs to one *kind*, and each air proves some subset of the kinds. The
//! kinds are handed out to the airs in priority order — most specific first — and each air takes what
//! fits in the instances the strategy gave it, leaving the rest pending for the next one. An operation
//! that no air ahead could take simply flows on, so a residual is never forced into an instance of its
//! own just because it did not fit in one place.
//!
//! Each kind is tracked separately, so what an instance collects is a `(count, skip)` per kind rather
//! than one figure over their union. That is what makes the hand-out independent of the order the kinds
//! happen to be interleaved in: the planner only ever needs counts, never the interleaving, and the
//! collector discovers the interleaving as it replays the chunk.
//!
//! This mirrors how the mem-align planner distributes its operation types, which also has to place
//! several kinds with different capacities across several airs.

use std::collections::HashMap;
use zisk_common::{ChunkId, CollectSkipper};

/// What one instance collects of one kind from one chunk.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KindCollect {
    /// Operations of the kind to collect.
    pub count: u64,

    /// Operations of the kind to let pass first: the share of the airs and instances ahead.
    pub skipper: CollectSkipper,

    /// Whether this instance accounts for this chunk's frequent operations of the kind.
    ///
    /// Frops take no row, so they are settled apart from the counts: for every chunk and kind exactly
    /// one instance is named accountant and counts all of them.
    pub owns_frops: bool,
}

impl KindCollect {
    fn taking(count: u64, skip: u64) -> Self {
        Self { count, skipper: CollectSkipper::new(skip), owns_frops: false }
    }
}

impl Default for KindCollect {
    /// Takes nothing of the kind and accounts for none of its frops.
    fn default() -> Self {
        Self::taking(0, 0)
    }
}

/// What one instance collects from one chunk, kind by kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChunkCollect<const K: usize> {
    pub kinds: [KindCollect; K],

    /// Whether the chunk must be walked to its end, which the accountant of any of its frops must do.
    pub force_execute_to_end: bool,
}

impl<const K: usize> Default for ChunkCollect<K> {
    fn default() -> Self {
        Self { kinds: [KindCollect::default(); K], force_execute_to_end: false }
    }
}

/// One air taking part in a distribution.
#[derive(Clone, Debug)]
pub struct AirSlot<const K: usize> {
    pub airgroup_id: usize,
    pub air_id: usize,

    /// Operations one instance of this air can hold.
    pub ops_per_instance: u64,

    /// Kinds this air is able to prove.
    pub proves: [bool; K],

    /// Instances the strategy granted it.
    pub instances: u64,
}

/// The per-chunk collects of one instance of one air.
#[derive(Debug)]
pub struct InstancePlan<const K: usize> {
    /// Index into the air slots this instance belongs to.
    pub air: usize,
    pub chunks: HashMap<ChunkId, ChunkCollect<K>>,
}

/// Hands out `ops` — real operations per chunk and kind — to `airs`, in the order they are given.
///
/// `frops` are the frequent operations per chunk and kind; they take no row, so they do not consume
/// capacity, but exactly one instance is named accountant for each (chunk, kind) that has any.
///
/// # Panics
/// Panics if the instances granted to the airs cannot hold every operation: the strategy that sized
/// them and this hand-out would then disagree, which would silently drop operations.
pub fn distribute<const K: usize>(
    ops: &[[u64; K]],
    frops: &[[u64; K]],
    airs: &[AirSlot<K>],
) -> Vec<InstancePlan<K>> {
    debug_assert_eq!(ops.len(), frops.len());

    let mut plans: Vec<InstancePlan<K>> = Vec::new();

    // Where each air currently is: the index of its open instance plan and the room left in it.
    let mut open: Vec<Option<usize>> = vec![None; airs.len()];
    let mut room: Vec<u64> = vec![0; airs.len()];
    let mut opened: Vec<u64> = vec![0; airs.len()];

    for (chunk, kinds) in ops.iter().enumerate() {
        let chunk_id = ChunkId(chunk);
        let mut pending = *kinds;

        for (a, air) in airs.iter().enumerate() {
            for k in 0..K {
                if !air.proves[k] {
                    continue;
                }
                while pending[k] > 0 {
                    if room[a] == 0 {
                        if opened[a] == air.instances {
                            break; // this air is full; the rest flows to the next one
                        }
                        plans.push(InstancePlan { air: a, chunks: HashMap::new() });
                        open[a] = Some(plans.len() - 1);
                        room[a] = air.ops_per_instance;
                        opened[a] += 1;
                    }

                    let take = pending[k].min(room[a]);
                    let skip = kinds[k] - pending[k];

                    let entry = plans[open[a].unwrap()].chunks.entry(chunk_id).or_default();
                    if entry.kinds[k].count == 0 {
                        entry.kinds[k] = KindCollect::taking(take, skip);
                    } else {
                        entry.kinds[k].count += take;
                    }

                    pending[k] -= take;
                    room[a] -= take;
                }
            }
        }

        assert!(
            pending.iter().all(|&p| p == 0),
            "chunk {chunk}: {pending:?} operations left unplaced; the instance counts and this \
             hand-out disagree"
        );
    }

    name_frops_accountants(frops, airs, &mut plans);
    plans
}

/// Names, for every chunk and kind that has frequent operations, the single instance accounting for
/// them: the last one that took operations of that kind there, or — when the chunk holds none of it —
/// the last instance of the first air able to prove it.
fn name_frops_accountants<const K: usize>(
    frops: &[[u64; K]],
    airs: &[AirSlot<K>],
    plans: &mut [InstancePlan<K>],
) {
    for (chunk, kinds) in frops.iter().enumerate() {
        let chunk_id = ChunkId(chunk);

        for (k, &count) in kinds.iter().enumerate() {
            if count == 0 {
                continue;
            }

            // The accountant must be able to see every frop of the kind, so it is the instance that
            // reaches the end of that kind in the chunk: the last one that took any of it.
            let last_taker = plans
                .iter()
                .enumerate()
                .rfind(|(_, p)| p.chunks.get(&chunk_id).is_some_and(|c| c.kinds[k].count > 0))
                .map(|(i, _)| i);

            // With no operation of the kind in the chunk there is no such instance, so it falls to the
            // first air that proves it — its last instance, which is the one that walks furthest.
            let accountant = last_taker.or_else(|| {
                let owner_air = airs.iter().position(|a| a.proves[k] && a.instances > 0)?;
                plans.iter().enumerate().rfind(|(_, p)| p.air == owner_air).map(|(i, _)| i)
            });

            let Some(accountant) = accountant else {
                panic!(
                    "chunk {chunk}: kind {k} has {count} frequent operations but no instance can \
                     account for them"
                );
            };

            let entry = plans[accountant].chunks.entry(chunk_id).or_default();
            entry.kinds[k].owns_frops = true;
            entry.force_execute_to_end = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn air<const K: usize>(
        air_id: usize,
        cap: u64,
        proves: [bool; K],
        instances: u64,
    ) -> AirSlot<K> {
        AirSlot { airgroup_id: 0, air_id, ops_per_instance: cap, proves, instances }
    }

    /// Sums, per chunk and kind, what every instance collects.
    fn collected<const K: usize>(plans: &[InstancePlan<K>], chunks: usize) -> Vec<[u64; K]> {
        let mut totals = vec![[0u64; K]; chunks];
        for plan in plans {
            for (chunk_id, c) in plan.chunks.iter() {
                for k in 0..K {
                    totals[chunk_id.0][k] += c.kinds[k].count;
                }
            }
        }
        totals
    }

    /// Every operation of every kind is collected exactly once, and the (skip, count) ranges of each
    /// kind tile it in order.
    fn assert_tiles<const K: usize>(plans: &[InstancePlan<K>], ops: &[[u64; K]]) {
        assert_eq!(collected(plans, ops.len()), ops, "operations lost or duplicated");

        for (chunk, kinds) in ops.iter().enumerate() {
            for k in 0..K {
                let mut ranges: Vec<(u64, u64)> = plans
                    .iter()
                    .filter_map(|p| p.chunks.get(&ChunkId(chunk)))
                    .filter(|c| c.kinds[k].count > 0)
                    .map(|c| (c.kinds[k].skipper.skip, c.kinds[k].count))
                    .collect();
                ranges.sort();
                let mut at = 0;
                for (skip, count) in ranges {
                    assert_eq!(skip, at, "chunk {chunk} kind {k}: gap or overlap at {skip}");
                    at += count;
                }
                assert_eq!(at, kinds[k], "chunk {chunk} kind {k}: not fully covered");
            }
        }
    }

    /// The most specific air fills up and the rest flows on, splitting a residual across the airs that
    /// follow — which is the whole point of handing the kinds out this way.
    #[test]
    fn a_residual_flows_on_and_may_split() {
        // Kinds: 0 = basic, 1 = low-limb add, 2 = full add.
        let ops = [[0, 10, 0], [0, 10, 0]];
        let frops = [[0, 0, 0], [0, 0, 0]];

        // The packed air holds 7 of them, then a dedicated air 5, then the general one the rest.
        let airs = [
            air(12, 7, [false, true, false], 1),
            air(11, 5, [false, true, true], 1),
            air(10, 100, [true, true, true], 1),
        ];

        let plans = distribute(&ops, &frops, &airs);
        assert_tiles(&plans, &ops);

        // 7 to the packed air, 5 to the dedicated one, 8 to the general one.
        let per_air = |id: usize| -> u64 {
            plans
                .iter()
                .filter(|p| airs[p.air].air_id == id)
                .flat_map(|p| p.chunks.values())
                .map(|c| c.kinds[1].count)
                .sum()
        };
        assert_eq!(per_air(12), 7);
        assert_eq!(per_air(11), 5);
        assert_eq!(per_air(10), 8);
    }

    /// A kind an air cannot prove flows past it untouched.
    #[test]
    fn an_air_never_takes_a_kind_it_cannot_prove() {
        let ops = [[4, 0, 6]];
        let frops = [[0, 0, 0]];
        let airs = [
            air(12, 100, [false, true, false], 1), // low-limb only: proves nothing here
            air(10, 100, [true, false, true], 1),
        ];

        let plans = distribute(&ops, &frops, &airs);
        assert_tiles(&plans, &ops);
        assert!(
            plans.iter().all(|p| airs[p.air].air_id != 12),
            "the packed air must not open an instance for kinds it cannot prove"
        );
    }

    /// One instance's capacity is respected, and the overflow opens the next one.
    #[test]
    fn instances_fill_in_order() {
        let ops = [[0, 0, 25]];
        let frops = [[0, 0, 0]];
        let airs = [air(11, 10, [false, false, true], 3)];

        let plans = distribute(&ops, &frops, &airs);
        assert_tiles(&plans, &ops);
        assert_eq!(plans.len(), 3);
        let counts: Vec<u64> = plans.iter().map(|p| p.chunks[&ChunkId(0)].kinds[2].count).collect();
        assert_eq!(counts, vec![10, 10, 5]);
    }

    /// Exactly one instance accounts for each chunk's frops of each kind, including when the chunk
    /// holds no real operation of that kind.
    #[test]
    fn exactly_one_accountant_per_chunk_and_kind() {
        let ops = [[3, 4, 0], [0, 0, 0]];
        let frops = [[2, 1, 5], [7, 0, 0]];
        let airs = [air(12, 2, [false, true, false], 1), air(10, 100, [true, true, true], 1)];

        let plans = distribute(&ops, &frops, &airs);
        assert_tiles(&plans, &ops);

        for (chunk, kinds) in frops.iter().enumerate() {
            for k in 0..3 {
                let owners = plans
                    .iter()
                    .filter(|p| {
                        p.chunks.get(&ChunkId(chunk)).is_some_and(|c| c.kinds[k].owns_frops)
                    })
                    .count();
                assert_eq!(
                    owners,
                    usize::from(kinds[k] > 0),
                    "chunk {chunk} kind {k} has {owners} accountants for {} frops",
                    kinds[k]
                );
            }
        }

        // An accountant always walks its chunk to the end.
        for plan in &plans {
            for c in plan.chunks.values() {
                if c.kinds.iter().any(|k| k.owns_frops) {
                    assert!(c.force_execute_to_end);
                }
            }
        }
    }

    /// The hand-out must not silently drop operations when the granted instances are too few.
    #[test]
    #[should_panic(expected = "left unplaced")]
    fn too_few_instances_is_an_error() {
        let ops = [[0, 0, 30]];
        let frops = [[0, 0, 0]];
        let airs = [air(11, 10, [false, false, true], 2)];
        distribute(&ops, &frops, &airs);
    }
}
