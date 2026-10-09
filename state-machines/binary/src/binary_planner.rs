//! The `BinaryPlanner` module defines a planner for generating execution plans specific to
//! binary operations (basic, extensions and dedicated adds)
//!
//! # Instance strategy
//!
//! Several airs can prove the same operation, at a different capacity and prover memory. Every one
//! of them comes in three sizes — a plain air, a `Large` and a `Huge` — which are all the same
//! height and differ in how many operations they pack on a row (`lanes_x_row` in the PIL):
//!
//! | air                              | proves                             | ops per instance    |
//! |----------------------------------|------------------------------------|---------------------|
//! | `Binary` / `…Large` / `…Huge`    | every basic op, additions included | rows × lanes        |
//! | `BinaryAdd` / `…Large` / `…Huge` | additions and SH3ADD of any shape it can carry | rows × lanes |
//! | `BinaryAddHi` / `…Large` / `…Huge` | low-limb additions and SH3ADD only | rows × lanes      |
//! | `BinaryExtension` / `…Large` / `…Huge` | every extension op           | rows × lanes        |
//!
//! The criterion is the shared one (see [`zisk_common::select_airs`]): **fewest instances first,
//! least prover memory to break a tie.** That is what makes the packing worth its width — one
//! `BinaryAddHiHuge` instance holds eight additions per row, eight times what a `Binary` instance
//! holds, so routing the additions there is what keeps the instance count down.
//!
//! Planning happens in two steps, which keeps the cost decision apart from the mechanics.
//!
//! **How many instances of each air.** Whole instances of the widest packed air are always worth
//! keeping — nothing holds more additions per instance — so the only thing to decide is what to do
//! with the operations left over, which is less than one of them. They can go to another instance of
//! the same air, to a couple of narrower packed ones, or ride in room already paid for by the basic
//! operations. Every combination is priced and the best wins, so nothing is hardcoded about which
//! air gives way.
//!
//! Note the narrower packed airs do NOT hold more than every other air — `BinaryAddHi` holds fewer
//! additions than `BinaryAddHuge` — which is exactly why they are enumerated as candidates rather
//! than assumed. Only the widest one dominates, and `tests::the_widest_packed_air_holds_the_most_
//! additions_per_instance` pins that.
//!
//! **Who collects what.** [`distribute`] then hands the operations to the airs in order, most
//! specialised and widest first, each taking what fits and leaving the rest pending for the next. A
//! residual is therefore never forced into an instance of its own merely because it did not fit in one
//! place: it can spread across every air that follows. The hand-out order matches the order the
//! strategy filled the airs in, which is what keeps the two consistent.
//!
//! Each kind of operation is tracked apart, so what an instance collects is a `(count, skip)` per
//! kind. The planner never needs to know the order the kinds are interleaved in — which it could not
//! know, having only counts — because each kind's boundary is expressed in that kind's own terms.
//! `SH3ADD` is split into its own kinds alongside the additions it shares an air with, since which
//! air can fold its shift into an addition depends on the operands (see [`crate::sh3add_shape`]).
//!
//! **The fused air.** `CompactBinary` carries lanes of the four airs side by side on one row (see
//! `pil/compact_binary.pil`), so one instance of it can hold a slice of every family at once: what
//! an execution that uses the binary airs lightly needs, instead of one instance per family. It is
//! priced under the same criterion: the layout with it -- the fused instance takes the minimum of
//! each family and its block, the rest is laid out as above -- against the layout without it, and
//! the one with fewer instances wins. When it is used, its blocks go FIRST in the hand-out, each
//! taking only the kinds its own air is the specialist of, so what the instance collects is exactly
//! the share it was sized with (see [`crate::compact_add_blocks`]).

use crate::{
    add_family, compact_add_blocks, compact_capacity, compact_ext_block, distribute, ext_family,
    lanes_x_row, AirSlot, BinaryCounter, ChunkCollect, CompactBinaryCollectInfo, InstancePlan,
    ADD_AIRS, ADD_KINDS, COMPACT_ADD_BLOCKS, COMPACT_BLOCK_ADD, COMPACT_BLOCK_ADD_HI,
    COMPACT_BLOCK_BASIC, EXT_AIRS, EXT_KINDS, KIND_ADD_FULL, KIND_ADD_HI, KIND_BASIC, KIND_EXT,
    KIND_SH3ADD_ADD, KIND_SH3ADD_HI,
};
use proofman_fields::PrimeField64;
use std::any::Any;
use zisk_common::{
    select_sizes, AirChoice, BusDeviceMetrics, CheckPoint, ChunkId, Cost, InstanceType, Metrics,
    Plan, Planner,
};
use zisk_pil::{
    BinaryAddHiHugeTrace, BinaryAddHiLargeTrace, BinaryAddHiTrace, BinaryAddHugeTrace,
    BinaryAddLargeTrace, BinaryAddTrace, BinaryExtensionLargeTrace, BinaryExtensionTrace,
    BinaryHugeTrace, BinaryLargeTrace, BinaryTrace, CompactBinaryTrace,
    BINARY_ADD_HI_HUGE_INSTANCE_COST, BINARY_ADD_HI_INSTANCE_COST,
    BINARY_ADD_HI_LARGE_INSTANCE_COST, BINARY_ADD_HUGE_INSTANCE_COST, BINARY_ADD_INSTANCE_COST,
    BINARY_ADD_LARGE_INSTANCE_COST, BINARY_EXTENSION_INSTANCE_COST,
    BINARY_EXTENSION_LARGE_INSTANCE_COST, BINARY_HUGE_INSTANCE_COST, BINARY_INSTANCE_COST,
    BINARY_LARGE_INSTANCE_COST, COMPACT_BINARY_INSTANCE_COST,
};

/// Slot of each air within [`add_family`] / [`InstanceCounts`], in hand-out order.
mod slot {
    /// `BinaryAddHiHuge`.
    pub const PACKED_HUGE: usize = 0;
    /// `BinaryAddHiLarge`.
    pub const PACKED_LARGE: usize = 1;
    /// `BinaryAddHi`.
    pub const PACKED: usize = 2;
    /// `BinaryAddHuge`.
    pub const ADD_HUGE: usize = 3;
    /// `BinaryAddLarge`.
    pub const ADD_LARGE: usize = 4;
    /// `BinaryAdd`.
    pub const ADD: usize = 5;
    /// `BinaryHuge`.
    pub const BASIC_HUGE: usize = 6;
    /// `BinaryLarge`.
    pub const BASIC_LARGE: usize = 7;
    /// `Binary`.
    pub const BASIC: usize = 8;
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

/// What the fused `CompactBinary` instance takes of each family: the minimum of the family and
/// its block, in the row cost of that block. The additions are the ones the packed airs would take
/// (`add_hi` with its SH3ADD shape) and the full ones (`add_full` with its SH3ADD shape), exactly
/// as [`Totals`] sizes them.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct CompactShare {
    basic: u64,
    add_hi: u64,
    add_full: u64,
    ext: u64,
}

impl CompactShare {
    fn of(totals: &Totals) -> Self {
        Self {
            basic: totals.basic.min(compact_capacity::basic()),
            add_hi: totals.add_hi.min(compact_capacity::add_hi()),
            add_full: totals.add_full.min(compact_capacity::add()),
            ext: totals.ext.min(compact_capacity::ext()),
        }
    }

    fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// What is left for the standalone airs once the fused instance took its share.
    fn remaining(&self, totals: &Totals) -> Totals {
        Totals {
            basic: totals.basic - self.basic,
            add_hi: totals.add_hi - self.add_hi,
            add_full: totals.add_full - self.add_full,
            ext: totals.ext - self.ext,
        }
    }
}

/// The instances a workload is laid out in: the add-family airs, the extension airs, and whether
/// one fused `CompactBinary` instance goes ahead of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    compact: bool,
    add: InstanceCounts,
    ext: [u64; EXT_AIRS],
}

/// Operations one instance of each add-family air holds, in [`slot`] order.
fn add_capacities() -> InstanceCounts {
    // Every air is the same height now, so what tells them apart is how many operations they pack
    // on a row: capacity is rows times lanes, not rows.
    let ops = |rows: usize, lanes: usize| (rows * lanes) as u64;
    [
        ops(BinaryAddHiHugeTrace::<()>::NUM_ROWS, lanes_x_row::ADD_HI_HUGE),
        ops(BinaryAddHiLargeTrace::<()>::NUM_ROWS, lanes_x_row::ADD_HI_LARGE),
        ops(BinaryAddHiTrace::<()>::NUM_ROWS, lanes_x_row::ADD_HI),
        ops(BinaryAddHugeTrace::<()>::NUM_ROWS, lanes_x_row::ADD_HUGE),
        ops(BinaryAddLargeTrace::<()>::NUM_ROWS, lanes_x_row::ADD_LARGE),
        ops(BinaryAddTrace::<()>::NUM_ROWS, lanes_x_row::ADD),
        ops(BinaryHugeTrace::<()>::NUM_ROWS, lanes_x_row::BASIC_HUGE),
        ops(BinaryLargeTrace::<()>::NUM_ROWS, lanes_x_row::BASIC_LARGE),
        ops(BinaryTrace::<()>::NUM_ROWS, lanes_x_row::BASIC),
    ]
}

/// Area of one instance of each add-family air, in [`slot`] order.
fn add_memories() -> InstanceCounts {
    [
        BINARY_ADD_HI_HUGE_INSTANCE_COST as u64,
        BINARY_ADD_HI_LARGE_INSTANCE_COST as u64,
        BINARY_ADD_HI_INSTANCE_COST as u64,
        BINARY_ADD_HUGE_INSTANCE_COST as u64,
        BINARY_ADD_LARGE_INSTANCE_COST as u64,
        BINARY_ADD_INSTANCE_COST as u64,
        BINARY_HUGE_INSTANCE_COST as u64,
        BINARY_LARGE_INSTANCE_COST as u64,
        BINARY_INSTANCE_COST as u64,
    ]
}

/// The two extension airs as a size ladder, tallest last so [`select_sizes`] can order them.
fn ext_ladder() -> [AirChoice; EXT_AIRS] {
    // `AirChoice::rows` is the capacity the choice offers, which for these airs is rows times the
    // lanes they pack: they are all the same height and differ only in the packing.
    let ops = |rows: usize, lanes: usize| rows * lanes;
    [
        AirChoice::new(
            BinaryExtensionLargeTrace::<()>::AIRGROUP_ID,
            BinaryExtensionLargeTrace::<()>::AIR_ID,
            ops(BinaryExtensionLargeTrace::<()>::NUM_ROWS, lanes_x_row::EXT_LARGE),
            BINARY_EXTENSION_LARGE_INSTANCE_COST,
        ),
        AirChoice::new(
            BinaryExtensionTrace::<()>::AIRGROUP_ID,
            BinaryExtensionTrace::<()>::AIR_ID,
            ops(BinaryExtensionTrace::<()>::NUM_ROWS, lanes_x_row::EXT),
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
        let memories = add_memories();
        Cost {
            instances: counts.iter().sum(),
            memory: counts.iter().zip(memories).map(|(&n, memory)| n * memory).sum(),
        }
    }

    /// What a whole layout costs: both families, the fused instance included when it is used.
    fn layout_cost(layout: &Layout) -> Cost {
        let mut cost = Self::cost_of(&layout.add);
        for (&n, air) in layout.ext.iter().zip(ext_ladder()) {
            cost.instances += n;
            cost.memory += n * air.memory;
        }
        if layout.compact {
            cost.instances += 1;
            cost.memory += COMPACT_BINARY_INSTANCE_COST as u64;
        }
        cost
    }

    /// Lays `totals` out without the fused air: the add family by [`best_add_counts`], the
    /// extension one by size.
    fn standalone_layout(totals: &Totals) -> Layout {
        let ext = select_sizes(totals.ext, &ext_ladder());
        Layout { compact: false, add: Self::best_add_counts(totals), ext: [ext[0], ext[1]] }
    }

    /// Picks between laying the workload out over the standalone airs alone and putting one fused
    /// `CompactBinary` instance ahead of them, which takes the minimum of each family and its
    /// block and leaves the rest to the standalone airs: fewest instances, then least memory.
    ///
    /// The fused air only ever pays off on a light workload -- several families with less than an
    /// instance each -- and this is what tells: a family big enough to fill its own instances
    /// loses nothing to the fused one, and gains an instance.
    fn best_layout(totals: &Totals) -> Layout {
        let without = Self::standalone_layout(totals);
        let share = CompactShare::of(totals);
        if share.is_empty() {
            return without;
        }
        let rest = Self::standalone_layout(&share.remaining(totals));
        let with = Layout { compact: true, ..rest };
        if Self::layout_cost(&with) < Self::layout_cost(&without) {
            with
        } else {
            without
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
        let memories = add_memories();
        // `AirChoice::rows` is the capacity the choice offers. These airs are all the same height,
        // so it is rows times the lanes they pack, which is what `add_capacities` already returns.
        let choice = |airgroup_id, air_id, slot: usize| AirChoice {
            airgroup_id,
            air_id,
            rows: caps[slot],
            memory: memories[slot],
        };
        let binary_ladder = [
            choice(
                BinaryHugeTrace::<()>::AIRGROUP_ID,
                BinaryHugeTrace::<()>::AIR_ID,
                slot::BASIC_HUGE,
            ),
            choice(
                BinaryLargeTrace::<()>::AIRGROUP_ID,
                BinaryLargeTrace::<()>::AIR_ID,
                slot::BASIC_LARGE,
            ),
            choice(BinaryTrace::<()>::AIRGROUP_ID, BinaryTrace::<()>::AIR_ID, slot::BASIC),
        ];
        let add_ladder = [
            choice(
                BinaryAddHugeTrace::<()>::AIRGROUP_ID,
                BinaryAddHugeTrace::<()>::AIR_ID,
                slot::ADD_HUGE,
            ),
            choice(
                BinaryAddLargeTrace::<()>::AIRGROUP_ID,
                BinaryAddLargeTrace::<()>::AIR_ID,
                slot::ADD_LARGE,
            ),
            choice(BinaryAddTrace::<()>::AIRGROUP_ID, BinaryAddTrace::<()>::AIR_ID, slot::ADD),
        ];

        let lay_out = |binary_ops: u64, add_ops: u64| -> InstanceCounts {
            let binary = select_sizes(binary_ops, &binary_ladder);
            let add = select_sizes(add_ops, &add_ladder);
            let mut counts = InstanceCounts::default();
            counts[slot::BASIC_HUGE] = binary[0];
            counts[slot::BASIC_LARGE] = binary[1];
            counts[slot::BASIC] = binary[2];
            counts[slot::ADD_HUGE] = add[0];
            counts[slot::ADD_LARGE] = add[1];
            counts[slot::ADD] = add[2];
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

    /// Picks how many instances of each add-family air to create: fewest instances, then least memory.
    ///
    /// The packed airs hold more additions per instance than anything else, so whole instances of
    /// them are never in question — only their leftover is. The candidates are therefore how many
    /// packed instances of each height to grant around that leftover, and for each the rest of the
    /// family is laid out by [`generic_counts`].
    fn best_add_counts(totals: &Totals) -> InstanceCounts {
        let caps = add_capacities();
        let (cap_huge, cap_large, cap_small) =
            (caps[slot::PACKED_HUGE], caps[slot::PACKED_LARGE], caps[slot::PACKED]);

        // Whole instances of the widest packed air are never in question: nothing holds more
        // additions per instance than it does. Only its leftover is, and that leftover is smaller
        // than one of them, so covering it takes at most one more of the same, or a couple of each
        // narrower packed air. Every such combination is priced and the cheapest wins.
        let whole_huge = totals.add_hi / cap_huge;

        [whole_huge, whole_huge + 1]
            .into_iter()
            .flat_map(|huge| {
                (0u64..=2).flat_map(move |large| (0u64..=2).map(move |small| (huge, large, small)))
            })
            .map(|(huge, large, small)| {
                // What each packed air actually receives, so no instance is granted room it cannot
                // use: an empty instance would only ever make the layout worse.
                let to_huge = totals.add_hi.min(huge * cap_huge);
                let to_large = (totals.add_hi - to_huge).min(large * cap_large);
                let to_small = (totals.add_hi - to_huge - to_large).min(small * cap_small);
                let rest = totals.add_hi - to_huge - to_large - to_small + totals.add_full;

                let mut counts = Self::generic_counts(totals.basic, rest);
                counts[slot::PACKED_HUGE] = to_huge.div_ceil(cap_huge);
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
    fn cover_frops<const K: usize>(frops: &[u64; K], airs: &mut [AirSlot<K>], memories: &[u64]) {
        for (k, &count) in frops.iter().enumerate() {
            if count == 0 || airs.iter().any(|a| a.sees[k] && a.instances > 0) {
                continue;
            }
            let smallest = airs
                .iter()
                .enumerate()
                .filter(|(_, a)| a.sees[k])
                .min_by_key(|(i, _)| memories[*i])
                .map(|(i, _)| i)
                .expect("every kind is seen by at least one air");
            airs[smallest].instances += 1;
        }
    }

    /// Turns the distribution of one family into plans.
    ///
    /// The first `compact_blocks` airs are the blocks of the fused `CompactBinary` air: what the
    /// hand-out gave them is returned apart, for the caller to merge into the one plan of the
    /// fused instance, since a block is not an air of its own.
    fn plans_of<const K: usize>(
        ops: &[[u64; K]],
        frops: &[[u64; K]],
        airs: &[AirSlot<K>],
        compact_blocks: usize,
    ) -> (Vec<Plan>, Vec<InstancePlan<K>>)
    where
        ChunkCollect<K>: Send + Sync + 'static,
    {
        let (compact, standalone): (Vec<_>, Vec<_>) = distribute(ops, frops, airs)
            .into_iter()
            .partition(|instance| instance.air < compact_blocks);
        let plans = standalone
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
            .collect();
        (plans, compact)
    }

    /// The one plan of the fused instance, from what the hand-out gave each of its blocks. `None`
    /// when no block got anything.
    fn compact_plan(
        add_blocks: Vec<InstancePlan<ADD_KINDS>>,
        ext_blocks: Vec<InstancePlan<EXT_KINDS>>,
    ) -> Option<Plan> {
        let mut info = CompactBinaryCollectInfo::default();
        for instance in add_blocks {
            let block = match instance.air {
                COMPACT_BLOCK_ADD_HI => &mut info.add_hi,
                COMPACT_BLOCK_ADD => &mut info.add,
                COMPACT_BLOCK_BASIC => &mut info.basic,
                air => unreachable!("air {air} is not a block of CompactBinary"),
            };
            // One instance per block at most: the strategy grants it one.
            block.extend(instance.chunks);
        }
        for instance in ext_blocks {
            debug_assert_eq!(instance.air, 0, "the ext block is the first air of its family");
            info.ext.extend(instance.chunks);
        }
        if info.is_empty() {
            return None;
        }
        let chunks = info.chunks();
        Some(Plan::new(
            CompactBinaryTrace::<()>::AIRGROUP_ID,
            CompactBinaryTrace::<()>::AIR_ID,
            None,
            InstanceType::Instance,
            CheckPoint::Multiple(chunks),
            Some(Box::new(info)),
        ))
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
            ops[KIND_SH3ADD_HI] = c.counter_sh3add_hi.inst_count;
            ops[KIND_SH3ADD_ADD] = c.counter_sh3add_add.inst_count;

            let mut fr = [0u64; ADD_KINDS];
            fr[KIND_BASIC] = c.counter_basic_wo_add.frops_count;
            fr[KIND_ADD_HI] = c.counter_add_hi.frops_count;
            fr[KIND_ADD_FULL] = c.counter_add.frops_count;
            fr[KIND_SH3ADD_HI] = c.counter_sh3add_hi.frops_count;
            fr[KIND_SH3ADD_ADD] = c.counter_sh3add_add.frops_count;

            let mut eops = [0u64; EXT_KINDS];
            eops[KIND_EXT] = c.counter_extension.inst_count;

            let mut efr = [0u64; EXT_KINDS];
            efr[KIND_EXT] = c.counter_extension.frops_count;

            totals.basic += ops[KIND_BASIC];
            totals.add_hi += ops[KIND_ADD_HI];
            totals.add_full += ops[KIND_ADD_FULL];
            // SH3ADD is sized alongside the additions it shares an air with: the Hi shape competes
            // for the packed airs, the Add shape for the full 64-bit ones.
            totals.add_hi += ops[KIND_SH3ADD_HI];
            totals.add_full += ops[KIND_SH3ADD_ADD];
            totals.ext += eops[KIND_EXT];

            add_ops.push(ops);
            add_frops.push(fr);
            ext_ops.push(eops);
            ext_frops.push(efr);
        }

        let layout = Self::best_layout(&totals);
        let compact_instances = layout.compact as u64;

        // The blocks of the fused air go first in each family's hand-out, so they take their
        // share and the standalone airs get what the strategy laid out for them.
        let mut add_airs: Vec<AirSlot<ADD_KINDS>> = compact_add_blocks(compact_instances)
            .into_iter()
            .chain(add_family(layout.add))
            .collect();
        let mut ext_airs: Vec<AirSlot<EXT_KINDS>> =
            std::iter::once(compact_ext_block(compact_instances))
                .chain(ext_family(layout.ext))
                .collect();

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
        // A block of the fused air costs the whole air, so it is never the smallest air to open
        // for coverage; and it needs no opening, it is there whenever the layout uses it.
        let add_areas: Vec<u64> = std::iter::repeat(COMPACT_BINARY_INSTANCE_COST as u64)
            .take(COMPACT_ADD_BLOCKS)
            .chain(add_memories())
            .collect();
        let ext_areas: Vec<u64> = std::iter::once(COMPACT_BINARY_INSTANCE_COST as u64)
            .chain(ext_ladder().iter().map(|air| air.memory))
            .collect();
        Self::cover_frops(&add_frops_total, &mut add_airs, &add_areas);
        Self::cover_frops(&ext_frops_total, &mut ext_airs, &ext_areas);

        tracing::debug!(
            "··· Binary instances: compact={} add_family={:?} ext={:?}",
            compact_instances,
            add_airs[COMPACT_ADD_BLOCKS..].iter().map(|a| a.instances).collect::<Vec<_>>(),
            ext_airs[1..].iter().map(|a| a.instances).collect::<Vec<_>>(),
        );

        let (mut plans, compact_add) =
            Self::plans_of(&add_ops, &add_frops, &add_airs, COMPACT_ADD_BLOCKS);
        let (mut ext_plans, compact_ext) = Self::plans_of(&ext_ops, &ext_frops, &ext_airs, 1);
        plans.append(&mut ext_plans);
        if let Some(plan) = Self::compact_plan(compact_add, compact_ext) {
            plans.push(plan);
        }
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

    /// The collects a plan carries: one add-family map and/or one extension map for a standalone
    /// air, the four blocks for the fused one.
    #[allow(clippy::type_complexity)]
    fn collects_of(
        meta: &Box<dyn Any + Send + Sync>,
    ) -> (
        Vec<&HashMap<ChunkId, ChunkCollect<ADD_KINDS>>>,
        Vec<&HashMap<ChunkId, ChunkCollect<EXT_KINDS>>>,
    ) {
        if let Some(cs) = meta.downcast_ref::<HashMap<ChunkId, ChunkCollect<ADD_KINDS>>>() {
            (vec![cs], vec![])
        } else if let Some(cs) = meta.downcast_ref::<HashMap<ChunkId, ChunkCollect<EXT_KINDS>>>() {
            (vec![], vec![cs])
        } else if let Some(info) = meta.downcast_ref::<CompactBinaryCollectInfo>() {
            (vec![&info.basic, &info.add, &info.add_hi], vec![&info.ext])
        } else {
            panic!("unexpected plan meta");
        }
    }

    fn totals(basic: u64, add_hi: u64, add_full: u64, ext: u64) -> Totals {
        Totals { basic, add_hi, add_full, ext }
    }

    /// The whole point of the fused air: a few operations of every family cost one instance
    /// instead of one per family.
    #[test]
    fn a_light_workload_takes_one_fused_instance() {
        let layout = TestPlanner::best_layout(&totals(10, 10, 10, 10));
        assert!(layout.compact);
        assert_eq!(layout.add, InstanceCounts::default(), "nothing is left for the add airs");
        assert_eq!(layout.ext, [0, 0], "nothing is left for the extension airs");
        assert_eq!(TestPlanner::layout_cost(&layout).instances, 1);

        // Without it the two families would need an instance each.
        let without = TestPlanner::standalone_layout(&totals(10, 10, 10, 10));
        assert_eq!(TestPlanner::layout_cost(&without).instances, 2);
    }

    /// A family that fills its own instances gains nothing from the fused air: it would still need
    /// those instances for what the block cannot hold, plus the fused one.
    #[test]
    fn a_heavy_family_keeps_its_own_airs() {
        let layout = TestPlanner::best_layout(&totals(cap(slot::BASIC_HUGE), 0, 0, 0));
        assert!(!layout.compact);
        assert_eq!(layout.add[slot::BASIC_HUGE], 1);

        let layout = TestPlanner::best_layout(&totals(0, 0, 0, 0));
        assert!(!layout.compact, "nothing to fuse");
    }

    /// The fused instance takes the minimum of each family and its block, and the rest is laid
    /// out as if the fused air did not exist.
    #[test]
    fn the_fused_instance_takes_its_share_and_the_rest_flows_on() {
        let totals = totals(compact_capacity::basic() + 5, 3, 0, compact_capacity::ext() + 1);
        let share = CompactShare::of(&totals);
        assert_eq!(share.basic, compact_capacity::basic());
        assert_eq!(share.add_hi, 3);
        assert_eq!(share.ext, compact_capacity::ext());
        assert_eq!(share.remaining(&totals), Totals { basic: 5, add_hi: 0, add_full: 0, ext: 1 });
    }

    /// The blocks of the fused air go first in the hand-out and every block keeps to its own
    /// kinds, so what the fused plan collects is exactly the share, chunk by chunk, and the
    /// standalone plans the rest.
    #[test]
    fn the_fused_plan_collects_exactly_the_share() {
        let boxed: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = (0..2)
            .map(|i| {
                let c = BinaryCounter {
                    counter_basic_wo_add: Counter { inst_count: 7, frops_count: 0 },
                    counter_sh3add_hi: Counter { inst_count: 1, frops_count: 0 },
                    counter_sh3add_add: Counter { inst_count: 2, frops_count: 0 },
                    counter_add_hi: Counter { inst_count: 4, frops_count: 0 },
                    counter_add: Counter { inst_count: 3, frops_count: 0 },
                    counter_extension: Counter { inst_count: 5, frops_count: 0 },
                };
                (ChunkId(i), Box::new(c) as Box<dyn BusDeviceMetrics>)
            })
            .collect();

        let plans = TestPlanner::new().plan(boxed);
        assert_eq!(plans.len(), 1, "a light workload is one fused instance: {plans:?}");
        let plan = &plans[0];
        assert_eq!(plan.air_id, CompactBinaryTrace::<()>::AIR_ID);
        let info = plan.meta.as_ref().unwrap().downcast_ref::<CompactBinaryCollectInfo>().unwrap();
        for chunk in 0..2 {
            let chunk = ChunkId(chunk);
            let kinds =
                |m: &HashMap<ChunkId, ChunkCollect<ADD_KINDS>>| m[&chunk].kinds.map(|k| k.count);
            assert_eq!(kinds(&info.basic), [7, 0, 0, 0, 0], "the basic block takes basic ops only");
            assert_eq!(kinds(&info.add_hi), [0, 4, 0, 1, 0], "the add_hi block, the low-limb ones");
            assert_eq!(kinds(&info.add), [0, 0, 3, 0, 2], "the add block, the full ones");
            assert_eq!(info.ext[&chunk].kinds.map(|k| k.count), [5]);
        }
        let CheckPoint::Multiple(chunks) = &plan.check_point else { panic!("multi-chunk") };
        assert_eq!(chunks, &[ChunkId(0), ChunkId(1)]);
    }

    /// The candidate set — keep the whole instances of the widest packed air, or one more — is only
    /// exhaustive because that air holds more additions per instance than any other. Were another air
    /// to overtake it, dropping below its whole instances could become worthwhile and this strategy
    /// would stop being optimal, so the ordering is pinned here.
    ///
    /// Note this is only claimed of the WIDEST packed air. The narrower packed ones no longer beat
    /// every other air — `BinaryAddHi` holds fewer additions than `BinaryAddHuge` or `BinaryHuge` —
    /// which is exactly why they are enumerated as candidates rather than assumed.
    #[test]
    fn the_widest_packed_air_holds_the_most_additions_per_instance() {
        let caps = add_capacities();
        for (other, cap) in caps.iter().enumerate() {
            if other == slot::PACKED_HUGE {
                continue;
            }
            assert!(
                caps[slot::PACKED_HUGE] > *cap,
                "the widest packed air must hold more additions per instance than air slot {other}",
            );
        }
    }

    /// The packed ladder is what `best_add_counts` enumerates a leftover over, and the range it
    /// offers (up to two of each narrower air) only covers that leftover because each tier is twice
    /// the one below it.
    #[test]
    fn the_packed_airs_form_a_doubling_ladder() {
        let caps = add_capacities();
        assert_eq!(caps[slot::PACKED_HUGE], 2 * caps[slot::PACKED_LARGE]);
        assert_eq!(caps[slot::PACKED_LARGE], 2 * caps[slot::PACKED]);
    }

    #[test]
    fn empty_totals_need_no_instances() {
        let counts = TestPlanner::best_add_counts(&Totals::default());
        assert_eq!(counts, InstanceCounts::default());
        assert_eq!(TestPlanner::cost_of(&counts), Cost::default());
        assert_eq!(select_sizes(0, &ext_ladder()), vec![0, 0]);
    }

    /// The whole point of the new criterion: work that would need two short instances is given one
    /// tall one instead, even though the memory is the same.
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

    /// Once the instance count is settled, memory decides: work that fits in the short air must not be
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

    /// The additions go to the packed airs, and within them to the widest one, which is what keeps
    /// the instance count down: the same additions in the `Binary` airs would need far more.
    #[test]
    fn the_additions_go_where_the_most_of_them_fit() {
        let add_hi = 4 * cap(slot::PACKED_HUGE);
        let counts = TestPlanner::best_add_counts(&Totals { add_hi, ..Default::default() });
        assert_eq!(counts[slot::PACKED_HUGE], 4, "the widest packed air takes them all");
        assert_eq!(counts[slot::PACKED_LARGE], 0);
        assert_eq!(counts[slot::PACKED], 0);
        assert_eq!(TestPlanner::cost_of(&counts).instances, 4);
        assert!(add_hi.div_ceil(cap(slot::BASIC_HUGE)) > 4, "the general air would need more");
    }

    /// A leftover smaller than the widest packed air is what the candidate enumeration is for: it
    /// must be able to land on a narrower packed air rather than force another wide instance.
    #[test]
    fn a_packed_leftover_lands_on_the_narrowest_air_that_holds_it() {
        // One whole wide instance plus a quarter of one, which is exactly one narrow instance.
        let add_hi = cap(slot::PACKED_HUGE) + cap(slot::PACKED);
        let counts = TestPlanner::best_add_counts(&Totals { add_hi, ..Default::default() });
        assert_eq!(counts[slot::PACKED_HUGE], 1, "the whole wide instance stays");
        assert_eq!(counts[slot::PACKED_LARGE], 0, "and the leftover does not need a wide one");
        assert_eq!(counts[slot::PACKED], 1, "the narrowest air that holds it takes the leftover");
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
        TestPlanner::cover_frops(&[4, 0, 0, 0, 0], &mut airs, &add_memories());
        assert_eq!(airs[slot::BASIC].instances, 1, "only the Binary airs see basic operations");
        assert_eq!(airs[slot::BASIC_LARGE].instances, 0, "and the cheaper of the two is enough");

        let mut airs = add_family(counts);
        TestPlanner::cover_frops(&[0, 4, 0, 0, 0], &mut airs, &add_memories());
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
                    counter_sh3add_hi: Counter { inst_count: 0, frops_count: 2 },
                    counter_sh3add_add: Counter { inst_count: 0, frops_count: 2 },
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

            let (add_blocks, ext_blocks) = collects_of(meta);
            for cs in add_blocks {
                for (chunk, c) in cs {
                    assert!(chunks.contains(chunk));
                    for (k, kind) in c.kinds.iter().enumerate() {
                        assert_eq!(kind.count, 0, "there is nothing to collect");
                        if kind.owns_frops {
                            *accountants.entry((0, chunk.0, k)).or_default() += 1;
                        }
                    }
                }
            }
            for cs in ext_blocks {
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
            (unit / 2, unit, unit / 4, 13, unit / 8, 3),
            (unit, 3 * unit, unit, 5, 0, unit / 2),
            (7, 5, 0, 11, 2, 1),
            (0, 0, 11, 0, 0, 0),
            (unit / 3, unit / 3, unit / 3, 3, unit / 3, unit / 3),
            (0, 4 * cap(slot::PACKED_LARGE), 0, 0, cap(slot::PACKED), 0),
        ];

        let boxed: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = shapes
            .iter()
            .enumerate()
            .map(|(i, &(basic, hi, full, ext, sh3_hi, sh3_add))| {
                let c = BinaryCounter {
                    counter_basic_wo_add: Counter { inst_count: basic, frops_count: 2 },
                    counter_sh3add_hi: Counter { inst_count: sh3_hi, frops_count: 2 },
                    counter_sh3add_add: Counter { inst_count: sh3_add, frops_count: 2 },
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
            let (add_blocks, ext_blocks) = collects_of(meta);
            for chunks in add_blocks {
                for (chunk, c) in chunks {
                    for (k, kind) in c.kinds.iter().enumerate() {
                        add_seen[chunk.0][k] += kind.count;
                        if kind.owns_frops {
                            *accountants.entry((0, chunk.0, k)).or_default() += 1;
                        }
                    }
                }
            }
            for chunks in ext_blocks {
                for (chunk, c) in chunks {
                    for (k, kind) in c.kinds.iter().enumerate() {
                        ext_seen[chunk.0][k] += kind.count;
                        if kind.owns_frops {
                            *accountants.entry((1, chunk.0, k)).or_default() += 1;
                        }
                    }
                }
            }
        }

        for (i, &(basic, hi, full, ext, sh3_hi, sh3_add)) in shapes.iter().enumerate() {
            assert_eq!(
                add_seen[i],
                [basic, hi, full, sh3_hi, sh3_add],
                "chunk {i}: add kinds not covered"
            );
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
