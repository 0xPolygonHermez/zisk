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

    /// Kinds this air is able to prove, and therefore to collect.
    pub proves: [bool; K],

    /// Kinds this air's collector *sees* on the bus, which is a broader set: it filters by operation
    /// type, not by the shape that decides the kind. Frequent operations take no row, so any of these
    /// can be accounted for here even when the air could not prove them.
    pub sees: [bool; K],

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
/// capacity, but exactly one instance is named accountant for each (chunk, kind) that has any. A kind
/// with frops and no operations at all still needs one, so an instance may be opened purely to account
/// for them — which is why the airs must be granted enough instances to cover every kind that has any.
///
/// # Panics
/// Panics if the instances granted to the airs cannot hold every operation, or cannot account for every
/// frequent one: the strategy that sized them and this hand-out would then disagree, which would
/// silently drop operations or leave frops uncounted.
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
/// the last instance of the first air whose collector sees the kind.
///
/// Seeing it is enough, since a frequent operation takes no row: an air that could not prove the kind
/// can still count it. That is what keeps an instance from being opened just to account for frops
/// whenever the family already has one.
fn name_frops_accountants<const K: usize>(
    frops: &[[u64; K]],
    airs: &[AirSlot<K>],
    plans: &mut Vec<InstancePlan<K>>,
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

            // With no operation of the kind in the chunk there is no such instance. An instance already
            // walking the chunk is then preferred, so accounting for these costs no extra replay: adding
            // the chunk to an instance that was not walking it would put it in that checkpoint too.
            let accountant = last_taker
                .or_else(|| {
                    plans
                        .iter()
                        .enumerate()
                        .rfind(|(_, p)| airs[p.air].sees[k] && p.chunks.contains_key(&chunk_id))
                        .map(|(i, _)| i)
                })
                .or_else(|| {
                    let owner_air = airs.iter().position(|a| a.sees[k] && a.instances > 0)?;
                    if let Some((i, _)) =
                        plans.iter().enumerate().rfind(|(_, p)| p.air == owner_air)
                    {
                        return Some(i);
                    }
                    plans.push(InstancePlan { air: owner_air, chunks: HashMap::new() });
                    Some(plans.len() - 1)
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
        AirSlot { airgroup_id: 0, air_id, ops_per_instance: cap, proves, sees: proves, instances }
    }

    /// Sums, per chunk and kind, what every instance collects.
    fn collected<const K: usize>(plans: &[InstancePlan<K>], chunks: usize) -> Vec<[u64; K]> {
        let mut totals = vec![[0u64; K]; chunks];
        for plan in plans {
            for (chunk_id, c) in plan.chunks.iter() {
                for (total, kind) in totals[chunk_id.0].iter_mut().zip(&c.kinds) {
                    *total += kind.count;
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
            for (k, &expected) in kinds.iter().enumerate() {
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
                assert_eq!(at, expected, "chunk {chunk} kind {k}: not fully covered");
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
            for (k, &count) in kinds.iter().enumerate() {
                let owners = plans
                    .iter()
                    .filter(|p| {
                        p.chunks.get(&ChunkId(chunk)).is_some_and(|c| c.kinds[k].owns_frops)
                    })
                    .count();
                assert_eq!(
                    owners,
                    usize::from(count > 0),
                    "chunk {chunk} kind {k} has {owners} accountants for {count} frops"
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

    /// A chunk holding only frequent operations of a kind still needs an accountant, even though it
    /// consumes no capacity. Reproduces the case an operation-count-only strategy leaves behind.
    #[test]
    fn frops_without_operations_still_get_an_accountant() {
        let ops = [[0, 0, 0]];
        let frops = [[0, 4, 0]];
        // The strategy saw no operations, so it granted the packed air one instance for the frops.
        let airs = [air(12, 10, [false, true, false], 1), air(10, 10, [true, true, true], 0)];

        let plans = distribute(&ops, &frops, &airs);
        let owners = plans
            .iter()
            .filter(|p| p.chunks.get(&ChunkId(0)).is_some_and(|c| c.kinds[1].owns_frops))
            .count();
        assert_eq!(owners, 1, "exactly one instance must account for them");
    }

    /// Seeing a kind is enough to account for its frequent operations, so an air that could not prove
    /// them still spares the family a whole instance. Here the packed air holds the low-limb additions
    /// and also counts the full-shape frops, which it cannot prove.
    #[test]
    fn an_air_accounts_for_a_kind_it_cannot_prove() {
        let ops = [[0, 6, 0]];
        let frops = [[0, 0, 3]];
        let airs = [
            AirSlot {
                airgroup_id: 0,
                air_id: 12,
                ops_per_instance: 10,
                proves: [false, true, false],
                sees: [false, true, true],
                instances: 1,
            },
            AirSlot {
                airgroup_id: 0,
                air_id: 11,
                ops_per_instance: 10,
                proves: [false, true, true],
                sees: [false, true, true],
                instances: 0,
            },
        ];

        let plans = distribute(&ops, &frops, &airs);
        assert_eq!(plans.len(), 1, "no instance is opened just for the frops");
        let c = plans[0].chunks[&ChunkId(0)];
        assert!(c.kinds[2].owns_frops, "the packed instance accounts for the full-shape frops");
        assert_eq!(c.kinds[2].count, 0, "without collecting any of them");
        assert!(c.force_execute_to_end);
    }

    /// A chunk holding frops but no operation of the kind is simply added to an existing instance,
    /// which then walks it. No instance is opened for it.
    #[test]
    fn a_frops_only_chunk_is_added_to_an_existing_instance() {
        // Operations live in chunk 0; chunk 2 has nothing but frops.
        let ops = [[0, 4, 0], [0, 0, 0], [0, 0, 0]];
        let frops = [[0, 0, 0], [0, 0, 0], [0, 3, 0]];
        let airs = [air(12, 10, [false, true, false], 1)];

        let plans = distribute(&ops, &frops, &airs);
        assert_eq!(plans.len(), 1, "no extra instance is opened for the frops-only chunk");

        let c = plans[0].chunks[&ChunkId(2)];
        assert_eq!(c.kinds[1].count, 0, "there is nothing to collect there");
        assert!(c.kinds[1].owns_frops, "but it does account for its frops");
        assert!(c.force_execute_to_end, "so it has to walk the chunk");
    }

    /// The accountant is picked among the instances already walking the chunk, so accounting for its
    /// frops adds no chunk to anyone's checkpoint. Here a second air already walks the chunk, while the
    /// last instance of the first air able to see the kind walks a different one.
    #[test]
    fn the_accountant_is_an_instance_already_walking_the_chunk() {
        // Kind 1 lives in chunk 1, kind 2 in chunk 0; only the second air sees both.
        let ops = [[0, 0, 5], [0, 6, 0]];
        let frops = [[0, 2, 0], [0, 0, 0]];
        let airs = [
            air(12, 10, [false, true, false], 1), // walks chunk 1
            AirSlot {
                airgroup_id: 0,
                air_id: 10,
                ops_per_instance: 10,
                proves: [true, true, true],
                sees: [true, true, true],
                instances: 1,
            }, // walks chunk 0
        ];

        let plans = distribute(&ops, &frops, &airs);
        let walks = |i: usize, chunk: usize| plans[i].chunks.contains_key(&ChunkId(chunk));

        // The frops of kind 1 in chunk 0 go to the instance already walking chunk 0...
        let accountant = plans
            .iter()
            .position(|p| p.chunks.get(&ChunkId(0)).is_some_and(|c| c.kinds[1].owns_frops))
            .expect("the frops must have an accountant");
        assert!(walks(accountant, 0), "and it was already walking that chunk");

        // ...so nobody gained a chunk it had no operations in.
        for (i, plan) in plans.iter().enumerate() {
            let has_ops = |chunk: usize| {
                plan.chunks
                    .get(&ChunkId(chunk))
                    .is_some_and(|c| c.kinds.iter().any(|k| k.count > 0))
            };
            for chunk in 0..2 {
                if walks(i, chunk) && !has_ops(chunk) {
                    assert_eq!(
                        i, accountant,
                        "only the accountant may walk a chunk it collects none of"
                    );
                }
            }
        }
    }

    /// At most one instance per family is ever opened purely for frops, since an air sees several kinds
    /// and covering one covers the rest.
    #[test]
    fn at_most_one_instance_is_opened_for_frops() {
        let ops = [[0, 0, 0]];
        let frops = [[5, 3, 2]];
        // The general air is the only one seeing every kind, and the strategy granted it one instance.
        let airs = [
            air(12, 10, [false, true, false], 0),
            air(11, 10, [false, true, true], 0),
            AirSlot {
                airgroup_id: 0,
                air_id: 10,
                ops_per_instance: 10,
                proves: [true, true, true],
                sees: [true, true, true],
                instances: 1,
            },
        ];

        let plans = distribute(&ops, &frops, &airs);
        assert_eq!(plans.len(), 1, "one instance accounts for all three kinds");
        let c = plans[0].chunks[&ChunkId(0)];
        assert!(c.kinds.iter().all(|k| k.owns_frops), "for every kind that has frops");
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
