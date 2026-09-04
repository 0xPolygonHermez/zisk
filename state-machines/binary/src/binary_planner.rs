//! The `BinaryPlanner` module defines a planner for generating execution plans specific to
//! binary operations (basic, extensions and dedicated adds)
//!
//! # Instance strategy
//!
//! Several airs can prove the same operation, at a different capacity and area. Every one of them
//! comes in two heights — a plain air and a `Large` sibling twice as tall and exactly as wide:
//!
//! | air                     | proves                              | ops per instance         |
//! |-------------------------|-------------------------------------|--------------------------|
//! | `Binary` / `…Large`     | every basic op, additions included  | rows                     |
//! | `BinaryAdd` / `…Large`  | additions of any shape              | rows                     |
//! | `BinaryAddHi`           | low-limb additions only             | rows × [`ADDS_X_ROW`]    |
//! | `BinaryAddHiLarge`      | low-limb additions only             | rows × [`ADDS_X_ROW_LARGE`] |
//! | `BinaryExtension` / `…Large` | every extension op             | rows                     |
//!
//! The criterion is the shared one (see [`zisk_common::select_airs`]): **fewest instances first, least
//! area to break a tie.** That is what makes the tall airs and the packing worth their width — one
//! `BinaryAddHiLarge` instance holds `ADDS_X_ROW_LARGE × 2²²` additions, two and a half times what a
//! `BinaryLarge` instance holds, so routing the additions there is what keeps the instance count down.
//!
//! Planning happens in two steps, which keeps the cost decision apart from the mechanics.
//!
//! **How many instances of each air.** Whole instances of the packed airs are always worth keeping —
//! nothing holds more additions per instance — so the only thing to decide is what to do with the
//! operations left over, which is at most one instance's worth. Giving them an instance of their own is
//! one option; the other is letting them ride in room already paid for by the basic operations. Both
//! are priced and the better wins, so nothing is hardcoded about which air gives way.
//!
//! **Who collects what.** [`distribute`] then hands the operations to the airs in order, most
//! specialised and tallest first, each taking what fits and leaving the rest pending for the next. A
//! residual is therefore never forced into an instance of its own merely because it did not fit in one
//! place: it can spread across every air that follows. The hand-out order matches the order the
//! strategy filled the airs in, which is what keeps the two consistent.
//!
//! Each kind of operation is tracked apart, so what an instance collects is a `(count, skip)` per
//! kind. The planner never needs to know the order the kinds are interleaved in — which it could not
//! know, having only counts — because each kind's boundary is expressed in that kind's own terms.

use crate::{
    add_family, distribute, ext_family, AirSlot, BinaryCounter, ChunkCollect, ADDS_X_ROW,
    ADDS_X_ROW_LARGE, ADD_AIRS, ADD_KINDS, EXT_AIRS, EXT_KINDS, KIND_ADD_FULL, KIND_ADD_HI,
    KIND_BASIC, KIND_EXT,
};
use proofman_fields::PrimeField64;
use std::any::Any;
use zisk_common::{
    select_sizes, AirChoice, BusDeviceMetrics, CheckPoint, ChunkId, Cost, InstanceType, Metrics,
    Plan, Planner,
};
use zisk_pil::{
    BinaryAddHiLargeTrace, BinaryAddHiTrace, BinaryAddLargeTrace, BinaryAddTrace,
    BinaryExtensionLargeTrace, BinaryExtensionTrace, BinaryLargeTrace, BinaryTrace,
    BINARY_ADD_HI_INSTANCE_COST, BINARY_ADD_HI_LARGE_INSTANCE_COST, BINARY_ADD_INSTANCE_COST,
    BINARY_ADD_LARGE_INSTANCE_COST, BINARY_EXTENSION_INSTANCE_COST,
    BINARY_EXTENSION_LARGE_INSTANCE_COST, BINARY_INSTANCE_COST, BINARY_LARGE_INSTANCE_COST,
};

/// Slot of each air within [`add_family`] / [`InstanceCounts`], in hand-out order.
mod slot {
    /// `BinaryAddHiLarge`.
    pub const PACKED_LARGE: usize = 0;
    /// `BinaryAddHi`.
    pub const PACKED: usize = 1;
    /// `BinaryAddLarge`.
    pub const ADD_LARGE: usize = 2;
    /// `BinaryAdd`.
    pub const ADD: usize = 3;
    /// `BinaryLarge`.
    pub const BASIC_LARGE: usize = 4;
    /// `Binary`.
    pub const BASIC: usize = 5;
}

/// Totals over every chunk, which is all the strategy needs.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct Totals {
    basic: u64,
    add_hi: u64,
    add_full: u64,
    ext: u64,
}

/// How many instances of each add-family air to create, in [`slot`] order.
type InstanceCounts = [u64; ADD_AIRS];

/// Operations one instance of each add-family air holds, in [`slot`] order.
fn add_capacities() -> InstanceCounts {
    [
        ADDS_X_ROW_LARGE as u64 * BinaryAddHiLargeTrace::<()>::NUM_ROWS as u64,
        ADDS_X_ROW as u64 * BinaryAddHiTrace::<()>::NUM_ROWS as u64,
        BinaryAddLargeTrace::<()>::NUM_ROWS as u64,
        BinaryAddTrace::<()>::NUM_ROWS as u64,
        BinaryLargeTrace::<()>::NUM_ROWS as u64,
        BinaryTrace::<()>::NUM_ROWS as u64,
    ]
}

/// Area of one instance of each add-family air, in [`slot`] order.
fn add_areas() -> InstanceCounts {
    [
        BINARY_ADD_HI_LARGE_INSTANCE_COST as u64,
        BINARY_ADD_HI_INSTANCE_COST as u64,
        BINARY_ADD_LARGE_INSTANCE_COST as u64,
        BINARY_ADD_INSTANCE_COST as u64,
        BINARY_LARGE_INSTANCE_COST as u64,
        BINARY_INSTANCE_COST as u64,
    ]
}

/// The two extension airs as a size ladder, tallest last so [`select_sizes`] can order them.
fn ext_ladder() -> [AirChoice; EXT_AIRS] {
    [
        AirChoice::new(
            BinaryExtensionLargeTrace::<()>::AIRGROUP_ID,
            BinaryExtensionLargeTrace::<()>::AIR_ID,
            BinaryExtensionLargeTrace::<()>::NUM_ROWS,
            BINARY_EXTENSION_LARGE_INSTANCE_COST,
        ),
        AirChoice::new(
            BinaryExtensionTrace::<()>::AIRGROUP_ID,
            BinaryExtensionTrace::<()>::AIR_ID,
            BinaryExtensionTrace::<()>::NUM_ROWS,
            BINARY_EXTENSION_INSTANCE_COST,
        ),
    ]
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

    /// What a layout costs, ranked the way the criterion ranks solutions.
    fn cost_of(counts: &InstanceCounts) -> Cost {
        let areas = add_areas();
        Cost {
            instances: counts.iter().sum(),
            area: counts.iter().zip(areas).map(|(&n, area)| n * area).sum(),
        }
    }

    /// Places the operations only the `Binary` airs can prove (`basic`) together with the additions
    /// no packed air took (`adds`), over the four airs that are left.
    ///
    /// Three layouts are worth considering and the best of them wins:
    ///
    /// * the additions ride in whatever room the `Binary` instances have left after the basic
    ///   operations, and only what does not fit takes a dedicated add instance;
    /// * the `Binary` airs swallow every addition too, which can spare a dedicated instance when the
    ///   additions are few;
    /// * the add airs take every addition, which is the cheaper home per operation when there are
    ///   enough of them to fill one.
    ///
    /// The first is what the hand-out actually performs when the counts allow it: [`distribute`] fills
    /// the add airs before the `Binary` ones, so granting them exactly what the `Binary` leftover
    /// cannot hold leaves precisely that leftover to ride along.
    fn generic_counts(basic: u64, adds: u64) -> InstanceCounts {
        let caps = add_capacities();
        let binary_ladder = [
            AirChoice {
                airgroup_id: BinaryLargeTrace::<()>::AIRGROUP_ID,
                air_id: BinaryLargeTrace::<()>::AIR_ID,
                rows: caps[slot::BASIC_LARGE],
                area: add_areas()[slot::BASIC_LARGE],
            },
            AirChoice {
                airgroup_id: BinaryTrace::<()>::AIRGROUP_ID,
                air_id: BinaryTrace::<()>::AIR_ID,
                rows: caps[slot::BASIC],
                area: add_areas()[slot::BASIC],
            },
        ];
        let add_ladder = [
            AirChoice {
                airgroup_id: BinaryAddLargeTrace::<()>::AIRGROUP_ID,
                air_id: BinaryAddLargeTrace::<()>::AIR_ID,
                rows: caps[slot::ADD_LARGE],
                area: add_areas()[slot::ADD_LARGE],
            },
            AirChoice {
                airgroup_id: BinaryAddTrace::<()>::AIRGROUP_ID,
                air_id: BinaryAddTrace::<()>::AIR_ID,
                rows: caps[slot::ADD],
                area: add_areas()[slot::ADD],
            },
        ];

        let lay_out = |binary_ops: u64, add_ops: u64| -> InstanceCounts {
            let binary = select_sizes(binary_ops, &binary_ladder);
            let add = select_sizes(add_ops, &add_ladder);
            let mut counts = InstanceCounts::default();
            counts[slot::BASIC_LARGE] = binary[0];
            counts[slot::BASIC] = binary[1];
            counts[slot::ADD_LARGE] = add[0];
            counts[slot::ADD] = add[1];
            counts
        };

        // Room the `Binary` instances that the basic operations force have left over.
        let for_basic = select_sizes(basic, &binary_ladder);
        let paid_room: u64 =
            for_basic.iter().zip(binary_ladder).map(|(&n, air)| n * air.rows).sum::<u64>() - basic;

        [
            lay_out(basic, adds.saturating_sub(paid_room)),
            lay_out(basic + adds, 0),
            lay_out(basic, adds),
        ]
        .into_iter()
        .min_by_key(|counts| Self::cost_of(counts))
        .expect("three layouts are always considered")
    }

    /// Picks how many instances of each add-family air to create: fewest instances, then least area.
    ///
    /// The packed airs hold more additions per instance than anything else, so whole instances of
    /// them are never in question — only their leftover is. The candidates are therefore how many
    /// packed instances of each height to grant around that leftover, and for each the rest of the
    /// family is laid out by [`generic_counts`].
    fn best_add_counts(totals: &Totals) -> InstanceCounts {
        let caps = add_capacities();
        let (cap_large, cap_small) = (caps[slot::PACKED_LARGE], caps[slot::PACKED]);

        let whole_large = totals.add_hi / cap_large;

        // One small packed instance may not hold the leftover of a large one, so two are offered.
        [whole_large, whole_large + 1]
            .into_iter()
            .flat_map(|large| [0u64, 1, 2].map(move |small| (large, small)))
            .map(|(large, small)| {
                // What each packed air actually receives, so no instance is granted room it cannot
                // use: an empty instance would only ever make the layout worse.
                let to_large = totals.add_hi.min(large * cap_large);
                let to_small = (totals.add_hi - to_large).min(small * cap_small);
                let rest = totals.add_hi - to_large - to_small + totals.add_full;

                let mut counts = Self::generic_counts(totals.basic, rest);
                counts[slot::PACKED_LARGE] = to_large.div_ceil(cap_large);
                counts[slot::PACKED] = to_small.div_ceil(cap_small);
                counts
            })
            .min_by_key(Self::cost_of)
            .expect("at least one candidate is always considered")
    }

    /// Makes sure every kind that has frequent operations has an air able to account for them.
    ///
    /// Frops take no row, so they do not enter the instance sizing at all: a family whose operations are
    /// all frequent gets no instance from it, leaving nobody to count them. An instance of the smallest
    /// air that *sees* the kind is opened for that — seeing it is enough, since counting a frequent
    /// operation takes no row — and only when no existing instance already sees it, so this is the last
    /// resort rather than the common path.
    ///
    /// Such an instance collects no operation at all. It is only reached when a whole family's
    /// operations are frequent, or when a kind only one air sees has none of its own.
    fn cover_frops<const K: usize>(frops: &[u64; K], airs: &mut [AirSlot<K>], areas: &[u64]) {
        for (k, &count) in frops.iter().enumerate() {
            if count == 0 || airs.iter().any(|a| a.sees[k] && a.instances > 0) {
                continue;
            }
            let smallest = airs
                .iter()
                .enumerate()
                .filter(|(_, a)| a.sees[k])
                .min_by_key(|(i, _)| areas[*i])
                .map(|(i, _)| i)
                .expect("every kind is seen by at least one air");
            airs[smallest].instances += 1;
        }
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
            eops[KIND_EXT] = c.counter_extension.inst_count;

            let mut efr = [0u64; EXT_KINDS];
            efr[KIND_EXT] = c.counter_extension.frops_count;

            totals.basic += ops[KIND_BASIC];
            totals.add_hi += ops[KIND_ADD_HI];
            totals.add_full += ops[KIND_ADD_FULL];
            totals.ext += eops[KIND_EXT];

            add_ops.push(ops);
            add_frops.push(fr);
            ext_ops.push(eops);
            ext_frops.push(efr);
        }

        let add_counts = Self::best_add_counts(&totals);
        let ext_counts = select_sizes(totals.ext, &ext_ladder());

        let mut add_airs = add_family(add_counts);
        let mut ext_airs = ext_family([ext_counts[0], ext_counts[1]]);

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
        let ext_areas: Vec<u64> = ext_ladder().iter().map(|air| air.area).collect();
        Self::cover_frops(&add_frops_total, &mut add_airs, &add_areas());
        Self::cover_frops(&ext_frops_total, &mut ext_airs, &ext_areas);

        tracing::debug!(
            "··· Binary instances: add_hi_large={} add_hi={} add_large={} add={} basic_large={} \
             basic={} ext_large={} ext={}",
            add_airs[0].instances,
            add_airs[1].instances,
            add_airs[2].instances,
            add_airs[3].instances,
            add_airs[4].instances,
            add_airs[5].instances,
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
    use proofman_fields::Goldilocks;
    use std::collections::HashMap;
    use zisk_common::Counter;

    type TestPlanner = BinaryPlanner<Goldilocks>;

    fn cap(slot: usize) -> u64 {
        add_capacities()[slot]
    }

    /// The candidate set — keep the whole packed instances, or one more — is only exhaustive because
    /// the packed airs hold more additions per instance than any other air. Were a taller `Binary` air
    /// to overtake them, dropping below the whole packed instances could become worthwhile and this
    /// strategy would stop being optimal, so the ordering is pinned here.
    #[test]
    fn the_packed_airs_hold_the_most_additions_per_instance() {
        let caps = add_capacities();
        for other in [slot::ADD_LARGE, slot::ADD, slot::BASIC_LARGE, slot::BASIC] {
            assert!(
                caps[slot::PACKED_LARGE] > caps[other],
                "the wide packed air must hold more additions per instance than air slot {other}",
            );
            assert!(
                caps[slot::PACKED] > caps[other],
                "the packed air must hold more additions per instance than air slot {other}",
            );
        }
    }

    #[test]
    fn empty_totals_need_no_instances() {
        let counts = TestPlanner::best_add_counts(&Totals::default());
        assert_eq!(counts, InstanceCounts::default());
        assert_eq!(TestPlanner::cost_of(&counts), Cost::default());
        assert_eq!(select_sizes(0, &ext_ladder()), vec![0, 0]);
    }

    /// The whole point of the new criterion: work that would need two short instances is given one
    /// tall one instead, even though the area is the same.
    #[test]
    fn one_tall_instance_beats_two_short_ones() {
        let counts = TestPlanner::best_add_counts(&Totals {
            basic: cap(slot::BASIC_LARGE),
            ..Default::default()
        });
        assert_eq!(counts[slot::BASIC_LARGE], 1);
        assert_eq!(counts[slot::BASIC], 0);
        assert_eq!(TestPlanner::cost_of(&counts).instances, 1);
    }

    /// Once the instance count is settled, area decides: work that fits in the short air must not be
    /// given the tall one.
    #[test]
    fn area_breaks_the_tie_between_the_two_heights() {
        let counts = TestPlanner::best_add_counts(&Totals { basic: 10, ..Default::default() });
        assert_eq!(counts[slot::BASIC], 1, "the short air is enough and is the cheaper one");
        assert_eq!(counts[slot::BASIC_LARGE], 0);
    }

    /// Additions ride in the leftover room of the `Binary` instances while there is any, so no
    /// dedicated instance is created for them.
    #[test]
    fn additions_fill_the_binary_leftover_first() {
        let counts =
            TestPlanner::best_add_counts(&Totals { basic: 10, add_hi: 10, add_full: 10, ext: 0 });
        assert_eq!(TestPlanner::cost_of(&counts).instances, 1, "one instance holds all of it");
        assert_eq!(counts[slot::PACKED] + counts[slot::PACKED_LARGE], 0);
        assert_eq!(counts[slot::ADD] + counts[slot::ADD_LARGE], 0);
    }

    /// Whole packed instances are kept, and the leftover rides in the `Binary` room rather than paying
    /// for an instance of its own.
    #[test]
    fn the_packed_leftover_rides_along() {
        let counts = TestPlanner::best_add_counts(&Totals {
            basic: 10,
            add_hi: cap(slot::PACKED_LARGE) + 5,
            ..Default::default()
        });
        assert_eq!(counts[slot::PACKED_LARGE], 1, "the whole packed instance stays");
        assert_eq!(TestPlanner::cost_of(&counts).instances, 2, "and one instance takes the rest");
        assert_eq!(counts[slot::PACKED], 0, "no second packed instance for five additions");
    }

    /// The additions go to the packed airs, which is what keeps the instance count down: the same
    /// additions in the `Binary` airs would need more than twice as many.
    #[test]
    fn the_additions_go_where_the_most_of_them_fit() {
        let add_hi = 4 * cap(slot::PACKED_LARGE);
        let counts = TestPlanner::best_add_counts(&Totals { add_hi, ..Default::default() });
        assert_eq!(counts[slot::PACKED_LARGE], 4);
        assert_eq!(TestPlanner::cost_of(&counts).instances, 4);
        assert!(add_hi.div_ceil(cap(slot::BASIC_LARGE)) > 4, "the general air would need more");
    }

    /// Additions that no packed air can prove still avoid the widest air when a narrower one holds
    /// them in the same number of instances.
    #[test]
    fn full_shape_additions_prefer_the_dedicated_air() {
        let counts = TestPlanner::best_add_counts(&Totals {
            add_full: cap(slot::ADD_LARGE),
            ..Default::default()
        });
        assert_eq!(counts[slot::ADD_LARGE], 1);
        assert_eq!(counts[slot::BASIC_LARGE], 0, "the general air is never opened for additions");
    }

    /// Frops of a kind no existing instance sees are the only reason to open one, and it is the
    /// smallest air that sees them: basic operations are only visible to the `Binary` airs, so one of
    /// their instances is unavoidable, whereas add frops ride in whatever add instance already exists.
    #[test]
    fn an_instance_is_opened_only_when_nothing_sees_the_kind() {
        let mut counts = InstanceCounts::default();
        counts[slot::ADD] = 1; // an add instance already exists
        let mut airs = add_family(counts);
        TestPlanner::cover_frops(&[4, 0, 0], &mut airs, &add_areas());
        assert_eq!(airs[slot::BASIC].instances, 1, "only the Binary airs see basic operations");
        assert_eq!(airs[slot::BASIC_LARGE].instances, 0, "and the cheaper of the two is enough");

        let mut airs = add_family(counts);
        TestPlanner::cover_frops(&[0, 4, 0], &mut airs, &add_areas());
        assert_eq!(airs.iter().map(|a| a.instances).sum::<u64>(), 1, "no instance is opened");
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

    /// End-to-end: the plans must cover every chunk of every air, kind by kind, and exactly one
    /// instance must account for each chunk's frops of each kind. This is also what proves the
    /// strategy and the hand-out agree — `distribute` panics when the granted instances cannot hold
    /// what the strategy routed to them.
    #[test]
    fn the_plans_cover_every_chunk_of_every_kind() {
        let unit = cap(slot::BASIC);
        let shapes = [
            (unit / 2, unit, unit / 4, 13),
            (unit, 3 * unit, unit, 5),
            (7, 5, 0, 11),
            (0, 0, 11, 0),
            (unit / 3, unit / 3, unit / 3, 3),
            (0, 4 * cap(slot::PACKED_LARGE), 0, 0),
        ];

        let boxed: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = shapes
            .iter()
            .enumerate()
            .map(|(i, &(basic, hi, full, ext))| {
                let c = BinaryCounter {
                    counter_basic_wo_add: Counter { inst_count: basic, frops_count: 2 },
                    counter_add_hi: Counter { inst_count: hi, frops_count: 1 },
                    counter_add: Counter { inst_count: full, frops_count: 3 },
                    counter_extension: Counter { inst_count: ext, frops_count: 1 },
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

        for (i, &(basic, hi, full, ext)) in shapes.iter().enumerate() {
            assert_eq!(add_seen[i], [basic, hi, full], "chunk {i}: add kinds not covered");
            assert_eq!(ext_seen[i], [ext], "chunk {i}: extension kinds not covered");

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
