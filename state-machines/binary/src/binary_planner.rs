//! The `BinaryPlanner` module defines a planner for generating execution plans specific to
//! binary operations (basic, extensions and dedicated adds)
//!
//! # Instance strategy
//!
//! Several airs can prove the same operation, at a different price and capacity:
//!
//! | air                   | proves                              | ops per instance      |
//! |-----------------------|-------------------------------------|-----------------------|
//! | `Binary`              | every basic op, additions included  | rows                  |
//! | `BinaryAdd`           | additions of any shape              | rows                  |
//! | `BinaryAddHi`         | low-limb additions only             | rows × [`ADDS_X_ROW`] |
//! | `BinaryExtension`     | extension ops with clean operands   | rows                  |
//! | `BinaryExtensionFull` | every extension op                  | rows                  |
//!
//! Planning happens in two steps, which keeps the cost decision apart from the mechanics.
//!
//! **How many instances of each air.** Whole instances of the specialised airs are always worth
//! keeping — one packed instance is cheaper than the [`ADDS_X_ROW`] plain ones it replaces — so the
//! only thing to decide is what to do with the operations left over, which is at most one instance's
//! worth. Giving them an instance of their own is one option; the other is letting them ride in room
//! already paid for. Both are priced and the cheaper wins, so nothing is hardcoded about which air
//! gives way.
//!
//! **Who collects what.** [`distribute`] then hands the operations to the airs in order, most
//! specialised first, each taking what fits and leaving the rest pending for the next. A residual is
//! therefore never forced into an instance of its own merely because it did not fit in one place: it
//! can spread across every air that follows.
//!
//! Each kind of operation is tracked apart, so what an instance collects is a `(count, skip)` per
//! kind. The planner never needs to know the order the kinds are interleaved in — which it could not
//! know, having only counts — because each kind's boundary is expressed in that kind's own terms.

use crate::{
    add_family, distribute, ext_family, AirSlot, BinaryCounter, ChunkCollect, ADD_KINDS, EXT_KINDS,
    KIND_ADD_FULL, KIND_ADD_HI, KIND_BASIC, KIND_EXT_CLEAN, KIND_EXT_DIRTY,
};
use proofman_fields::PrimeField64;
use std::any::Any;
use zisk_common::{BusDeviceMetrics, CheckPoint, ChunkId, InstanceType, Metrics, Plan, Planner};

/// Columns of each binary air per row, as the setup reports them.
///
/// This is the per-air weight the planner cannot derive from the PIL alone, so it is declared here.
/// It is the figure the executor already prices instances with, namely the sum of
/// `stark_info.map_sections_n` over every section but `const` — see `setup_cost` in
/// `executor/src/adapters.rs`:
///
/// ```text
/// cost = (1 << stark_info.stark_struct.n_bits) * total_cols
/// ```
///
/// `1 << n_bits` is the air's rows, taken from its trace, so what is left to declare is `total_cols`.
/// Note it counts the stage and auxiliary columns too, which is why these values exceed the committed
/// width of the trace row: the `Binary` row commits 39 field elements but its instance is priced at 60
/// per row.
///
/// **These come from a generated setup, so they have to be refreshed whenever an air's columns
/// change.** `weights_cover_the_committed_columns` catches the most likely form of staleness — a
/// weight that no longer even covers the trace's committed width — but it cannot see the stage
/// columns, so a setup that grows only those will not be flagged.
mod columns {
    pub const BINARY: u64 = 60;
    pub const BINARY_ADD: u64 = 25;
    pub const BINARY_ADD_HI: u64 = 36;
    pub const BINARY_EXTENSION: u64 = 52;
    pub const BINARY_EXTENSION_FULL: u64 = 58;
}

/// Totals over every chunk, which is all the strategy needs.
#[derive(Default, Clone, Copy, Debug)]
struct Totals {
    basic: u64,
    add_hi: u64,
    add_full: u64,
    ext_clean: u64,
    ext_dirty: u64,
}

/// How many instances of each air to create.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct InstanceCounts {
    basic: u64,
    add: u64,
    add_hi: u64,
    ext: u64,
    ext_full: u64,
}

impl InstanceCounts {
    fn total(&self) -> u64 {
        self.basic + self.add + self.add_hi + self.ext + self.ext_full
    }
}

/// The `BinaryPlanner` struct organizes execution plans for binaries instances and tables.
#[derive(Default)]
pub struct BinaryPlanner<F> {
    _marker: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> BinaryPlanner<F> {
    pub fn new() -> Self {
        Self { _marker: std::marker::PhantomData }
    }

    /// Rows of each air per instance, which is what its cost is proportional to.
    fn rows() -> (u64, u64, u64, u64, u64) {
        use zisk_pil::{
            BinaryAddHiTrace, BinaryAddTrace, BinaryExtensionFullTrace, BinaryExtensionTrace,
            BinaryTrace,
        };
        (
            BinaryTrace::<()>::NUM_ROWS as u64,
            BinaryAddTrace::<()>::NUM_ROWS as u64,
            BinaryAddHiTrace::<()>::NUM_ROWS as u64,
            BinaryExtensionTrace::<()>::NUM_ROWS as u64,
            BinaryExtensionFullTrace::<()>::NUM_ROWS as u64,
        )
    }

    /// Cost of one instance of each add-family air, in slot order.
    fn add_instance_costs() -> [u64; 3] {
        let (basic, add, add_hi, _, _) = Self::rows();
        [add_hi * columns::BINARY_ADD_HI, add * columns::BINARY_ADD, basic * columns::BINARY]
    }

    /// Cost of one instance of each extension-family air, in slot order.
    fn ext_instance_costs() -> [u64; 2] {
        let (_, _, _, ext, ext_full) = Self::rows();
        [ext * columns::BINARY_EXTENSION, ext_full * columns::BINARY_EXTENSION_FULL]
    }

    /// Makes sure every kind that has frequent operations has an air able to account for them.
    ///
    /// Frops take no row, so they do not enter the instance sizing at all: a family whose operations are
    /// all frequent gets no instance from it, leaving nobody to count them. An instance of the cheapest
    /// air that *sees* the kind is opened for that — seeing it is enough, since counting a frequent
    /// operation takes no row — and only when no existing instance already sees it, so this is the last
    /// resort rather than the common path.
    ///
    /// Such an instance collects no operation at all. It is only reached when a whole family's
    /// operations are frequent, or when a kind only one air sees has none of its own.
    fn cover_frops<const K: usize>(frops: &[u64; K], airs: &mut [AirSlot<K>], costs: &[u64]) {
        for (k, &count) in frops.iter().enumerate() {
            if count == 0 || airs.iter().any(|a| a.sees[k] && a.instances > 0) {
                continue;
            }
            let cheapest = airs
                .iter()
                .enumerate()
                .filter(|(_, a)| a.sees[k])
                .min_by_key(|(i, _)| costs[*i])
                .map(|(i, _)| i)
                .expect("every kind is seen by at least one air");
            airs[cheapest].instances += 1;
        }
    }

    /// Cost of the layout described by `counts`: each air's columns times its rows, per instance.
    fn cost_of(counts: &InstanceCounts) -> u64 {
        let (basic, add, add_hi, ext, ext_full) = Self::rows();
        counts.basic * basic * columns::BINARY
            + counts.add * add * columns::BINARY_ADD
            + counts.add_hi * add_hi * columns::BINARY_ADD_HI
            + counts.ext * ext * columns::BINARY_EXTENSION
            + counts.ext_full * ext_full * columns::BINARY_EXTENSION_FULL
    }

    /// Instance counts for the add family, given how many packed instances to keep.
    ///
    /// The `Binary` instances are forced by the basic operations, so the room left over in them is
    /// already paid for; the additions the specialised airs did not take fill it before any new
    /// instance is created.
    fn add_counts(totals: &Totals, packed: u64, airs: &[AirSlot<ADD_KINDS>; 3]) -> InstanceCounts {
        let hi_taken = totals.add_hi.min(packed * airs[0].ops_per_instance);
        let mut rest = (totals.add_hi - hi_taken) + totals.add_full;

        let basic = totals.basic.div_ceil(airs[2].ops_per_instance);
        let free = basic * airs[2].ops_per_instance - totals.basic;
        rest -= rest.min(free);

        InstanceCounts {
            basic,
            add: rest.div_ceil(airs[1].ops_per_instance),
            add_hi: packed,
            ..Default::default()
        }
    }

    /// Instance counts for the extension family, given how many reduced instances to keep.
    fn ext_counts(totals: &Totals, reduced: u64, airs: &[AirSlot<EXT_KINDS>; 2]) -> InstanceCounts {
        let clean_taken = totals.ext_clean.min(reduced * airs[0].ops_per_instance);
        let rest = (totals.ext_clean - clean_taken) + totals.ext_dirty;

        InstanceCounts {
            ext: reduced,
            ext_full: rest.div_ceil(airs[1].ops_per_instance),
            ..Default::default()
        }
    }

    /// Picks how many instances of each air to create: cheapest first, then fewest instances.
    ///
    /// Only the leftover of each specialised air is in question, so the candidates are "keep the whole
    /// instances" and "one more to absorb the leftover" — anything below the whole instances is dearer,
    /// since one packed instance always beats the plain ones it replaces.
    fn best_counts(totals: &Totals) -> InstanceCounts {
        let add_airs = add_family(0, 0, 0);
        let ext_airs = ext_family(0, 0);

        let whole_packed = totals.add_hi / add_airs[0].ops_per_instance;
        let whole_reduced = totals.ext_clean / ext_airs[0].ops_per_instance;

        let mut best: Option<InstanceCounts> = None;
        for packed in [whole_packed, whole_packed + 1] {
            for reduced in [whole_reduced, whole_reduced + 1] {
                let add = Self::add_counts(totals, packed, &add_airs);
                let ext = Self::ext_counts(totals, reduced, &ext_airs);
                let counts = InstanceCounts {
                    basic: add.basic,
                    add: add.add,
                    add_hi: add.add_hi,
                    ext: ext.ext,
                    ext_full: ext.ext_full,
                };
                let better = match best {
                    None => true,
                    Some(b) => {
                        (Self::cost_of(&counts), counts.total()) < (Self::cost_of(&b), b.total())
                    }
                };
                if better {
                    best = Some(counts);
                }
            }
        }
        best.expect("at least one candidate is always considered")
    }

    /// Turns the distribution of one family into plans.
    fn plans_of<const K: usize>(
        ops: &[[u64; K]],
        frops: &[[u64; K]],
        airs: &[AirSlot<K>],
    ) -> Vec<Plan>
    where
        ChunkCollect<K>: Send + Sync + 'static,
    {
        distribute(ops, frops, airs)
            .into_iter()
            .map(|instance| {
                let air = &airs[instance.air];
                let chunks: Vec<ChunkId> = instance.chunks.keys().cloned().collect();
                let meta: Box<dyn Any + Send + Sync> = Box::new(instance.chunks);
                Plan::new(
                    air.airgroup_id,
                    air.air_id,
                    None,
                    InstanceType::Instance,
                    CheckPoint::Multiple(chunks),
                    Some(meta),
                )
            })
            .collect()
    }
}

impl<F: PrimeField64> Planner for BinaryPlanner<F> {
    /// Generates execution plans for binary instances.
    ///
    /// # Panics
    /// Panics if any counter cannot be downcasted to a `BinaryCounter`.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let binary: Vec<&BinaryCounter> = counters
            .iter()
            .map(|(_, c)| Metrics::as_any(&**c).downcast_ref::<BinaryCounter>().unwrap())
            .collect();

        // Per-chunk operations and frops of each kind, in chunk order.
        let mut add_ops = Vec::with_capacity(binary.len());
        let mut add_frops = Vec::with_capacity(binary.len());
        let mut ext_ops = Vec::with_capacity(binary.len());
        let mut ext_frops = Vec::with_capacity(binary.len());
        let mut totals = Totals::default();

        for c in &binary {
            let mut ops = [0u64; ADD_KINDS];
            ops[KIND_BASIC] = c.counter_basic_wo_add.inst_count;
            ops[KIND_ADD_HI] = c.counter_add_hi.inst_count;
            ops[KIND_ADD_FULL] = c.counter_add.inst_count;

            let mut fr = [0u64; ADD_KINDS];
            fr[KIND_BASIC] = c.counter_basic_wo_add.frops_count;
            fr[KIND_ADD_HI] = c.counter_add_hi.frops_count;
            fr[KIND_ADD_FULL] = c.counter_add.frops_count;

            let mut eops = [0u64; EXT_KINDS];
            eops[KIND_EXT_CLEAN] = c.counter_extension.inst_count;
            eops[KIND_EXT_DIRTY] = c.counter_extension_full.inst_count;

            let mut efr = [0u64; EXT_KINDS];
            efr[KIND_EXT_CLEAN] = c.counter_extension.frops_count;
            efr[KIND_EXT_DIRTY] = c.counter_extension_full.frops_count;

            totals.basic += ops[KIND_BASIC];
            totals.add_hi += ops[KIND_ADD_HI];
            totals.add_full += ops[KIND_ADD_FULL];
            totals.ext_clean += eops[KIND_EXT_CLEAN];
            totals.ext_dirty += eops[KIND_EXT_DIRTY];

            add_ops.push(ops);
            add_frops.push(fr);
            ext_ops.push(eops);
            ext_frops.push(efr);
        }

        let counts = Self::best_counts(&totals);

        let mut add_airs = add_family(counts.add_hi, counts.add, counts.basic);
        let mut ext_airs = ext_family(counts.ext, counts.ext_full);

        // The sizing above only saw operations. A kind whose operations are all frequent would be left
        // with no air to account for them, so coverage is topped up here.
        let mut add_frops_total = [0u64; ADD_KINDS];
        for f in &add_frops {
            for (total, count) in add_frops_total.iter_mut().zip(f) {
                *total += count;
            }
        }
        let mut ext_frops_total = [0u64; EXT_KINDS];
        for f in &ext_frops {
            for (total, count) in ext_frops_total.iter_mut().zip(f) {
                *total += count;
            }
        }
        Self::cover_frops(&add_frops_total, &mut add_airs, &Self::add_instance_costs());
        Self::cover_frops(&ext_frops_total, &mut ext_airs, &Self::ext_instance_costs());

        tracing::debug!(
            "··· Binary instances: add_hi={} add={} basic={} ext={} ext_full={}",
            add_airs[0].instances,
            add_airs[1].instances,
            add_airs[2].instances,
            ext_airs[0].instances,
            ext_airs[1].instances,
        );

        let mut plans = Self::plans_of(&add_ops, &add_frops, &add_airs);
        plans.append(&mut Self::plans_of(&ext_ops, &ext_frops, &ext_airs));
        plans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use zisk_common::Counter;
    use zisk_pil::{
        BinaryAddHiTrace, BinaryAddHiTraceRow, BinaryAddTrace, BinaryAddTraceRow,
        BinaryExtensionFullTrace, BinaryExtensionFullTraceRow, BinaryExtensionTrace,
        BinaryExtensionTraceRow, BinaryTrace, BinaryTraceRow,
    };

    type TestPlanner = BinaryPlanner<proofman_fields::Goldilocks>;

    fn cap_basic() -> u64 {
        BinaryTrace::<()>::NUM_ROWS as u64
    }
    fn cap_packed() -> u64 {
        crate::ADDS_X_ROW as u64 * BinaryAddHiTrace::<()>::NUM_ROWS as u64
    }

    /// The declared weights come from a generated setup, so they cannot be checked against the PIL
    /// exactly. What must hold is that each covers at least the committed width of its trace row — the
    /// setup counts those columns plus the stage and auxiliary ones.
    #[test]
    fn weights_cover_the_committed_columns() {
        type F = proofman_fields::Goldilocks;
        for (name, weight, row_size) in [
            ("Binary", columns::BINARY, BinaryTrace::<BinaryTraceRow<F>>::ROW_SIZE),
            ("BinaryAdd", columns::BINARY_ADD, BinaryAddTrace::<BinaryAddTraceRow<F>>::ROW_SIZE),
            (
                "BinaryAddHi",
                columns::BINARY_ADD_HI,
                BinaryAddHiTrace::<BinaryAddHiTraceRow<F>>::ROW_SIZE,
            ),
            (
                "BinaryExtension",
                columns::BINARY_EXTENSION,
                BinaryExtensionTrace::<BinaryExtensionTraceRow<F>>::ROW_SIZE,
            ),
            (
                "BinaryExtensionFull",
                columns::BINARY_EXTENSION_FULL,
                BinaryExtensionFullTrace::<BinaryExtensionFullTraceRow<F>>::ROW_SIZE,
            ),
        ] {
            assert!(
                weight >= row_size as u64,
                "the setup weight of {name} ({weight}) no longer covers its {row_size} committed \
                 columns: the air gained columns and the weight was not refreshed"
            );
        }
    }

    /// The candidate set — keep the whole specialised instances, or one more — is only exhaustive
    /// because each specialised air is the cheapest **per operation** of those that can prove its kind.
    /// Were a refreshed setup weight to flip that, dropping below the whole instances could become
    /// worthwhile and this strategy would stop being optimal, so the ordering is pinned here.
    #[test]
    fn the_specialised_airs_are_the_cheapest_per_operation() {
        let (basic, add, add_hi, ext, ext_full) = TestPlanner::rows();
        let per_op = |instances_cost: u64, ops: u64| (instances_cost, ops);

        // Packed additions against the plain add air, and both against the general one.
        let packed = per_op(add_hi * columns::BINARY_ADD_HI, cap_packed());
        let plain = per_op(add * columns::BINARY_ADD, add);
        let general = per_op(basic * columns::BINARY, basic);
        assert!(
            packed.0 * plain.1 < plain.0 * packed.1,
            "the packed air must be cheaper per operation than the plain add air"
        );
        assert!(
            plain.0 * general.1 < general.0 * plain.1,
            "the plain add air must be cheaper per operation than the general one"
        );

        // Clean extension operations against the full air.
        let reduced = per_op(ext * columns::BINARY_EXTENSION, ext);
        let full = per_op(ext_full * columns::BINARY_EXTENSION_FULL, ext_full);
        assert!(
            reduced.0 * full.1 < full.0 * reduced.1,
            "the reduced extension air must be cheaper per operation than the full one"
        );
    }

    /// Nothing to prove, nothing to plan.
    #[test]
    fn empty_totals_need_no_instances() {
        let counts = TestPlanner::best_counts(&Totals::default());
        assert_eq!(counts, InstanceCounts::default());
        assert_eq!(TestPlanner::cost_of(&counts), 0);
    }

    /// Additions ride in the leftover room of the `Binary` instances while there is any, so no
    /// dedicated instance is created for them.
    #[test]
    fn additions_fill_the_binary_leftover_first() {
        let counts = TestPlanner::best_counts(&Totals {
            basic: 10,
            add_hi: 10,
            add_full: 10,
            ..Default::default()
        });
        assert_eq!(counts.basic, 1);
        assert_eq!(counts.add, 0, "no dedicated add instance is needed");
        assert_eq!(counts.add_hi, 0);
    }

    /// Once that room is gone the additions go to the cheapest air that can hold them.
    #[test]
    fn additions_leave_binary_once_it_is_full() {
        let counts = TestPlanner::best_counts(&Totals {
            basic: cap_basic(),
            add_hi: 10,
            add_full: 10,
            ..Default::default()
        });
        assert_eq!(counts.basic, 1);
        assert!(counts.add + counts.add_hi > 0);
    }

    /// Whole packed instances are kept, and the leftover rides in the `Binary` room rather than paying
    /// for a partial packed instance.
    #[test]
    fn the_packed_leftover_rides_along() {
        let counts = TestPlanner::best_counts(&Totals {
            basic: 10,
            add_hi: cap_packed() + 5,
            ..Default::default()
        });
        assert_eq!(counts.add_hi, 1, "the whole packed instance stays");
        assert_eq!(counts.basic, 1);
        assert_eq!(counts.add, 0, "the 5 left over ride with the basic operations");
    }

    /// With no free room to ride in, a small leftover goes to the cheapest air that can hold it rather
    /// than opening a second instance of the packed one.
    #[test]
    fn a_small_leftover_goes_to_the_cheapest_air() {
        // No basic operations, so `Binary` has no room to give.
        let counts =
            TestPlanner::best_counts(&Totals { add_hi: cap_packed() + 7, ..Default::default() });
        assert_eq!(counts.add_hi, 1, "one whole packed instance");
        assert_eq!(counts.basic, 0, "the general air is never opened just for additions");
        assert_eq!(counts.add, 1, "the 7 left over take a cheap dedicated instance");

        // And that really is the cheaper of the two options.
        let second_packed = InstanceCounts { add_hi: 2, ..Default::default() };
        assert!(TestPlanner::cost_of(&counts) < TestPlanner::cost_of(&second_packed));
    }

    /// A leftover large enough is better served by another packed instance, since the packed air is the
    /// cheapest per operation. Which way it goes is decided by cost, not by a fixed preference.
    #[test]
    fn a_large_leftover_earns_another_packed_instance() {
        let counts = TestPlanner::best_counts(&Totals {
            add_hi: cap_packed() + cap_basic() + 7,
            ..Default::default()
        });
        assert_eq!(counts.add_hi, 2, "a second packed instance beats several plain ones");
        assert_eq!(counts.add, 0);

        let spread = InstanceCounts { add_hi: 1, add: 2, ..Default::default() };
        assert!(TestPlanner::cost_of(&counts) < TestPlanner::cost_of(&spread));
    }

    /// Clean extension operations get the same treatment: whole reduced instances are kept and the
    /// leftover rides in the full air, which is needed for the dirty ones anyway. Without this the
    /// planner had to choose between an extra reduced instance and pricing every clean operation at
    /// the full air's rate.
    #[test]
    fn the_clean_extension_leftover_rides_in_the_full_air() {
        let cap = BinaryExtensionTrace::<()>::NUM_ROWS as u64;
        let counts = TestPlanner::best_counts(&Totals {
            ext_clean: 7 * cap + cap / 4,
            ext_dirty: cap / 4,
            ..Default::default()
        });
        assert_eq!(counts.ext, 7, "the whole reduced instances stay");
        assert_eq!(counts.ext_full, 1, "the leftover and the dirty ones share one full instance");

        // Cheaper than either all-or-nothing option.
        let all_full = InstanceCounts { ext_full: 8, ..Default::default() };
        let one_more_reduced = InstanceCounts { ext: 8, ext_full: 1, ..Default::default() };
        assert!(TestPlanner::cost_of(&counts) < TestPlanner::cost_of(&all_full));
        assert!(TestPlanner::cost_of(&counts) < TestPlanner::cost_of(&one_more_reduced));
    }

    /// Extension operations that all need the full air must not create a reduced instance.
    #[test]
    fn no_reduced_extension_instance_when_all_operands_are_dirty() {
        let counts =
            TestPlanner::best_counts(&Totals { ext_dirty: 1_000_000, ..Default::default() });
        assert_eq!(counts.ext, 0);
        assert_eq!(counts.ext_full, 1);
    }

    /// A workload whose binary operations are *all* frequent still has to be planned: the frops
    /// multiplicities have to be counted or the frequent-operations lookup will not balance, and only a
    /// collector can count them. The sizing sees no operations, so this is the one case where a binary
    /// instance ends up collecting none — which the state machines handle by padding the whole trace.
    #[test]
    fn a_frops_only_workload_still_gets_accountants() {
        let boxed: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = (0..3)
            .map(|i| {
                let c = BinaryCounter {
                    counter_basic_wo_add: Counter { inst_count: 0, frops_count: 4 },
                    counter_add_hi: Counter { inst_count: 0, frops_count: 2 },
                    counter_add: Counter { inst_count: 0, frops_count: 3 },
                    counter_extension: Counter { inst_count: 0, frops_count: 5 },
                    counter_extension_full: Counter { inst_count: 0, frops_count: 1 },
                };
                (ChunkId(i), Box::new(c) as Box<dyn BusDeviceMetrics>)
            })
            .collect();

        let plans = TestPlanner::new().plan(boxed);
        assert!(!plans.is_empty(), "the frops still need an accountant");

        let mut accountants: HashMap<(usize, usize, usize), usize> = HashMap::new();
        for plan in &plans {
            let meta = plan.meta.as_ref().unwrap();
            let CheckPoint::Multiple(chunks) = &plan.check_point else {
                panic!("expected a multi-chunk checkpoint");
            };
            assert!(!chunks.is_empty(), "an instance with no chunk would never run");

            if let Some(cs) = meta.downcast_ref::<HashMap<ChunkId, ChunkCollect<ADD_KINDS>>>() {
                for (chunk, c) in cs {
                    assert!(chunks.contains(chunk));
                    for (k, kind) in c.kinds.iter().enumerate() {
                        assert_eq!(kind.count, 0, "there is nothing to collect");
                        if kind.owns_frops {
                            *accountants.entry((0, chunk.0, k)).or_default() += 1;
                        }
                    }
                }
            } else if let Some(cs) =
                meta.downcast_ref::<HashMap<ChunkId, ChunkCollect<EXT_KINDS>>>()
            {
                for (chunk, c) in cs {
                    assert!(chunks.contains(chunk));
                    for (k, kind) in c.kinds.iter().enumerate() {
                        assert_eq!(kind.count, 0);
                        if kind.owns_frops {
                            *accountants.entry((1, chunk.0, k)).or_default() += 1;
                        }
                    }
                }
            }
        }

        for chunk in 0..3 {
            for k in 0..ADD_KINDS {
                assert_eq!(accountants.get(&(0, chunk, k)), Some(&1), "chunk {chunk} add kind {k}");
            }
            for k in 0..EXT_KINDS {
                assert_eq!(accountants.get(&(1, chunk, k)), Some(&1), "chunk {chunk} ext kind {k}");
            }
        }
    }

    /// Frops of a kind no existing instance sees are the only reason to open one, and it is the cheapest
    /// air that sees them: basic operations are only visible to `Binary`, so its instance is unavoidable,
    /// whereas add frops ride in whatever add instance already exists.
    #[test]
    fn an_instance_is_opened_only_when_nothing_sees_the_kind() {
        let mut airs = add_family(0, 1, 0); // an add instance already exists
        TestPlanner::cover_frops(&[4, 0, 0], &mut airs, &TestPlanner::add_instance_costs());
        assert_eq!(airs[2].instances, 1, "only Binary sees basic operations");

        // Add frops, by contrast, are already visible to the existing add instance.
        let mut airs = add_family(0, 1, 0);
        TestPlanner::cover_frops(&[0, 4, 0], &mut airs, &TestPlanner::add_instance_costs());
        assert_eq!(airs.iter().map(|a| a.instances).sum::<u64>(), 1, "no instance is opened");
    }

    /// End-to-end: the plans must cover every chunk of every air, kind by kind, and exactly one
    /// instance must account for each chunk's frops of each kind.
    #[test]
    fn the_plans_cover_every_chunk_of_every_kind() {
        let cap = cap_basic();
        let shapes = [
            (cap / 2, cap, cap / 4, 13, 17),
            (cap, 3 * cap, cap, 0, 5),
            (7, 5, 0, 11, 0),
            (0, 0, 11, 0, 0),
            (cap / 3, cap / 3, cap / 3, 3, 3),
        ];

        let boxed: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = shapes
            .iter()
            .enumerate()
            .map(|(i, &(basic, hi, full, clean, dirty))| {
                let c = BinaryCounter {
                    counter_basic_wo_add: Counter { inst_count: basic, frops_count: 2 },
                    counter_add_hi: Counter { inst_count: hi, frops_count: 1 },
                    counter_add: Counter { inst_count: full, frops_count: 3 },
                    counter_extension: Counter { inst_count: clean, frops_count: 1 },
                    counter_extension_full: Counter { inst_count: dirty, frops_count: 1 },
                };
                (ChunkId(i), Box::new(c) as Box<dyn BusDeviceMetrics>)
            })
            .collect();

        let plans = TestPlanner::new().plan(boxed);

        let mut add_seen = vec![[0u64; ADD_KINDS]; shapes.len()];
        let mut ext_seen = vec![[0u64; EXT_KINDS]; shapes.len()];
        let mut accountants: HashMap<(usize, usize, usize), usize> = HashMap::new();

        for plan in &plans {
            let meta = plan.meta.as_ref().expect("every plan carries its collects");
            if let Some(chunks) = meta.downcast_ref::<HashMap<ChunkId, ChunkCollect<ADD_KINDS>>>() {
                for (chunk, c) in chunks {
                    for (k, kind) in c.kinds.iter().enumerate() {
                        add_seen[chunk.0][k] += kind.count;
                        if kind.owns_frops {
                            *accountants.entry((0, chunk.0, k)).or_default() += 1;
                        }
                    }
                }
            } else if let Some(chunks) =
                meta.downcast_ref::<HashMap<ChunkId, ChunkCollect<EXT_KINDS>>>()
            {
                for (chunk, c) in chunks {
                    for (k, kind) in c.kinds.iter().enumerate() {
                        ext_seen[chunk.0][k] += kind.count;
                        if kind.owns_frops {
                            *accountants.entry((1, chunk.0, k)).or_default() += 1;
                        }
                    }
                }
            } else {
                panic!("unexpected plan meta");
            }
        }

        for (i, &(basic, hi, full, clean, dirty)) in shapes.iter().enumerate() {
            assert_eq!(add_seen[i], [basic, hi, full], "chunk {i}: add kinds not covered");
            assert_eq!(ext_seen[i], [clean, dirty], "chunk {i}: extension kinds not covered");

            // Every chunk here has frops of every kind, so each needs exactly one accountant.
            for k in 0..ADD_KINDS {
                assert_eq!(accountants.get(&(0, i, k)), Some(&1), "chunk {i} add kind {k}");
            }
            for k in 0..EXT_KINDS {
                assert_eq!(accountants.get(&(1, i, k)), Some(&1), "chunk {i} ext kind {k}");
            }
        }
    }
}
