//! The `BinaryPlanner` module defines a planner for generating execution plans specific to
//! binary operations (basic, extensions and dedicated adds)
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging binary operation counts by operation and metadata to construct detailed plans.
//!
//! # Instance strategy
//!
//! Several airs can prove the same operation, and they are not interchangeable:
//!
//! | air                   | proves                              | ops per row     |
//! |-----------------------|-------------------------------------|-----------------|
//! | `Binary`              | every basic op, additions included  | 1               |
//! | `BinaryAdd`           | additions of any shape              | 1               |
//! | `BinaryAddHi`         | low-limb additions only             | [`ADDS_X_ROW`]  |
//! | `BinaryExtension`     | extension ops with clean operands   | 1               |
//! | `BinaryExtensionFull` | every extension op                  | 1               |
//!
//! The strategy is to **fill the most specific air first** and leave only what it cannot prove to
//! the more general alternatives: low-limb additions prefer the packed `BinaryAddHi`, the remaining
//! additions prefer the dedicated `BinaryAdd` over the much wider `Binary`, and extension operations
//! with clean operands prefer the reduced `BinaryExtension` over `BinaryExtensionFull`.
//!
//! That preference is not unconditional, because every instance is a fixed-size air: a handful of
//! operations sent to their own air still pay for a whole instance, so riding along in an air that
//! is needed anyway and still has room is free. The planner therefore enumerates the few possible
//! routings (see [`Routing::all`]) and ranks them by, in order: total cost (each air priced by its
//! own [`AirBudget`], i.e. its committed [`columns`] times its rows), then number of instances, then
//! how specific the routing is. Ties thus resolve towards fewer instances first and towards the more
//! specific air second.
//!
//! Two consequences worth knowing, both pinned by tests: an operation only moves to its dedicated
//! air once the general one would need an extra instance for it, and the packed `BinaryAddHi` only
//! beats `BinaryAdd` past one plain instance, since it is cheaper per operation but its rows are
//! wider.
//!
//! Routings are chosen globally rather than per chunk so that an operation's air does not depend on
//! where it happens to fall, which keeps the counters, the collectors and this planner in agreement.

use crate::{rows_needed, BinaryCollectInfo, BinaryCounter, ExtensionScope, ShapeDrop, ADDS_X_ROW};
use fields::PrimeField64;
use std::any::Any;
use std::collections::HashMap;
use zisk_common::{
    plan_with_frops, BusDeviceMetrics, ChunkId, InstFropsCount, InstanceType, Metrics, Plan,
    Planner,
};
use zisk_pil::{
    BinaryAddHiTrace, BinaryAddTrace, BinaryExtensionFullTrace, BinaryExtensionTrace, BinaryTrace,
};

/// Total committed columns of each binary air.
///
/// This is the per-air weight the planner cannot derive from anything else, so it is declared
/// explicitly here: one instance is not worth the same as another, a `Binary` row commits about four
/// times as much as a `BinaryAdd` row. The cost of one instance is this number times the rows of
/// that instance (see [`AirBudget::new`]).
///
/// They are plain constants, gathered here, so the balance between airs can be re-tuned if measured
/// proving cost ever diverges from committed width. The `columns_match_the_traces` test pins them to
/// the current trace geometry so they cannot drift unnoticed.
mod columns {
    pub const BINARY: u64 = 39;
    pub const BINARY_ADD: u64 = 10;
    pub const BINARY_ADD_HI: u64 = 15;
    pub const BINARY_EXTENSION: u64 = 28;
    pub const BINARY_EXTENSION_FULL: u64 = 34;
}

/// Capacity and price of one instance of a given air.
#[derive(Clone, Copy, Debug)]
struct AirBudget {
    /// Operations one instance can prove.
    ops_per_instance: u64,

    /// Cost of one instance: its total committed columns times its rows.
    cost_per_instance: u64,
}

impl AirBudget {
    /// Budget of an air with `num_rows` rows per instance, `columns` committed columns and
    /// `ops_per_row` operations proven per row.
    ///
    /// Both figures are a function of `num_rows`, which is read from the air's own trace rather than
    /// assumed: the airs need not be instantiated with the same N, so each one is priced and sized
    /// by its own.
    const fn new(num_rows: usize, columns: u64, ops_per_row: u64) -> Self {
        Self {
            ops_per_instance: num_rows as u64 * ops_per_row,
            cost_per_instance: num_rows as u64 * columns,
        }
    }

    /// Instances needed to prove `ops` operations, and what they cost.
    fn instances_and_cost(&self, ops: u64) -> (u64, u64) {
        let instances = ops.div_ceil(self.ops_per_instance);
        (instances, instances * self.cost_per_instance)
    }
}

/// Where the additions that need the full 64-bit add ([`crate::AddShape::Full`]) are proven.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FullAddRoute {
    /// Sent to the dedicated `BinaryAdd` air, the most specific one able to prove them.
    Add,
    /// Merged into the general `Binary` air, so no dedicated add instance is created for them.
    Basic,
}

/// Where the additions whose result fits in the low limb ([`crate::AddShape::Hi`] and
/// [`crate::AddShape::HiNeg`]) are proven.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HiAddRoute {
    /// Sent to the packed `BinaryAddHi` air, [`ADDS_X_ROW`] per row: the most specific one.
    AddHi,
    /// Sent to the dedicated `BinaryAdd` air, one per row.
    Add,
    /// Merged into the general `Binary` air.
    Basic,
}

/// How the extension operations are split between the reduced and the full air.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ExtRoute {
    /// Clean operands go to the reduced `BinaryExtension`, the rest to `BinaryExtensionFull`.
    Split,
    /// Everything goes to `BinaryExtensionFull`, so no reduced instance is created.
    FullOnly,
}

/// One candidate assignment of operations to airs.
#[derive(Clone, Copy, Debug)]
struct Routing {
    full_add: FullAddRoute,
    hi_add: HiAddRoute,
    ext: ExtRoute,
}

/// Where the residual additions of a shape end up once the whole dedicated instances are taken.
///
/// Whole instances of a dedicated air are already optimal, so the only thing left to decide is where
/// the residual goes: into room another air has already paid for, or into an instance of its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TailHome {
    /// The leftover room of the `Binary` instances, paid for by the basic operations.
    Basic,
    /// A `BinaryAdd` instance, shared with the other residual when both land there.
    Add,
    /// A `BinaryAddHi` instance of its own.
    AddHi,
}

/// Where each shape's residual goes.
#[derive(Clone, Copy, Debug)]
struct TailHomes {
    full: TailHome,
    hi: TailHome,
}

impl TailHomes {
    /// Every combination worth trying. `BinaryAddHi` cannot prove full-shape additions, so that
    /// residual only has two homes.
    fn all() -> impl Iterator<Item = TailHomes> {
        const FULL: [TailHome; 2] = [TailHome::Add, TailHome::Basic];
        const HI: [TailHome; 3] = [TailHome::AddHi, TailHome::Add, TailHome::Basic];

        FULL.into_iter().flat_map(|full| HI.into_iter().map(move |hi| TailHomes { full, hi }))
    }
}

/// How many operations each binary air proves under a given routing, and what that costs.
#[derive(Default, Clone, Copy, Debug)]
struct BinaryLayout {
    basic_ops: u64,
    add_ops: u64,
    add_hi_ops: u64,
    extension_ops: u64,
    extension_full_ops: u64,

    /// Full-shape additions `BinaryAdd` takes, as a prefix of that shape's sequence. Whatever is
    /// left over goes to `Binary`.
    full_to_add: u64,

    /// Low-limb additions `BinaryAddHi` takes, as a prefix of that shape's sequence.
    hi_to_add_hi: u64,

    /// Low-limb additions `BinaryAdd` takes, right after `BinaryAddHi`'s prefix. Whatever is left
    /// over after both goes to `Binary`.
    hi_to_add: u64,

    /// Total number of instances across every air.
    instances: u64,

    /// Total cost of those instances.
    cost: u64,
}

impl Routing {
    /// Every routing the planner considers, listed from the most specific to the least, so that
    /// ties resolve towards keeping operations in their dedicated air.
    fn all() -> impl Iterator<Item = Routing> {
        const FULL_ADD: [FullAddRoute; 2] = [FullAddRoute::Add, FullAddRoute::Basic];
        const HI_ADD: [HiAddRoute; 3] = [HiAddRoute::AddHi, HiAddRoute::Add, HiAddRoute::Basic];
        const EXT: [ExtRoute; 2] = [ExtRoute::Split, ExtRoute::FullOnly];

        FULL_ADD.into_iter().flat_map(|full_add| {
            HI_ADD.into_iter().flat_map(move |hi_add| {
                EXT.into_iter().map(move |ext| Routing { full_add, hi_add, ext })
            })
        })
    }
}

/// Totals over every chunk, used to pick a routing.
#[derive(Default, Clone, Copy, Debug)]
struct Totals {
    basic_wo_add: u64,
    add: u64,
    add_hi: u64,
    extension: u64,
    extension_full: u64,
}

/// The `BinaryPlanner` struct organizes execution plans for binaries instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct BinaryPlanner<F> {
    _marker: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> BinaryPlanner<F> {
    pub fn new() -> Self {
        Self { _marker: std::marker::PhantomData }
    }

    fn basic_budget() -> AirBudget {
        AirBudget::new(BinaryTrace::<()>::NUM_ROWS, columns::BINARY, 1)
    }

    fn add_budget() -> AirBudget {
        AirBudget::new(BinaryAddTrace::<()>::NUM_ROWS, columns::BINARY_ADD, 1)
    }

    /// The packed air proves [`ADDS_X_ROW`] operations per row, so its capacity in operations is
    /// that many times its row count.
    fn add_hi_budget() -> AirBudget {
        AirBudget::new(BinaryAddHiTrace::<()>::NUM_ROWS, columns::BINARY_ADD_HI, ADDS_X_ROW as u64)
    }

    fn extension_budget() -> AirBudget {
        AirBudget::new(BinaryExtensionTrace::<()>::NUM_ROWS, columns::BINARY_EXTENSION, 1)
    }

    fn extension_full_budget() -> AirBudget {
        AirBudget::new(BinaryExtensionFullTrace::<()>::NUM_ROWS, columns::BINARY_EXTENSION_FULL, 1)
    }

    /// Splits a shape's per-chunk counts at global operation `boundary`.
    ///
    /// Returns, per chunk, how many of its operations fall before the boundary. Chunks are walked in
    /// order, which is the order the collectors see them, so each side of a boundary ends up with a
    /// contiguous range of the shape's sequence and a plain skip separates them.
    fn dedicated_share(counts: &[u64], boundary: u64) -> Vec<u64> {
        let mut left = boundary;
        counts
            .iter()
            .map(|&n| {
                let take = n.min(left);
                left -= take;
                take
            })
            .collect()
    }

    /// Operations, instances and cost of each air under `routing`, placing each shape's residual in
    /// `tails`. Returns `None` when that placement is not viable.
    ///
    /// The shape of the decision follows from two facts. Whole instances of a dedicated air are
    /// already optimal, so nothing is decided there; and the `Binary` instances are forced by the
    /// basic operations, so the room left over in them is already paid for. What is left to decide is
    /// only where each shape's residual goes — into that free room, into an instance another residual
    /// is already paying for, or into one of its own.
    fn layout_of(totals: &Totals, routing: &Routing, tails: &TailHomes) -> Option<BinaryLayout> {
        let basic = Self::basic_budget();
        let add = Self::add_budget();
        let add_hi = Self::add_hi_budget();

        let full_has_dedicated = routing.full_add == FullAddRoute::Add;
        let hi_has_dedicated = routing.hi_add != HiAddRoute::Basic;

        let mut layout = BinaryLayout::default();

        // Shapes routed to the general air have no dedicated instance at all.
        let mut basic_ops = totals.basic_wo_add;
        if !full_has_dedicated {
            basic_ops += totals.add;
        }
        if !hi_has_dedicated {
            basic_ops += totals.add_hi;
        }

        // Whole instances of each dedicated air, and the residual each one leaves behind.
        let (full_whole, full_tail) = if full_has_dedicated {
            (totals.add - totals.add % add.ops_per_instance, totals.add % add.ops_per_instance)
        } else {
            (0, 0)
        };
        let hi_budget = if routing.hi_add == HiAddRoute::AddHi { add_hi } else { add };
        let (hi_whole, hi_tail) = if hi_has_dedicated {
            (
                totals.add_hi - totals.add_hi % hi_budget.ops_per_instance,
                totals.add_hi % hi_budget.ops_per_instance,
            )
        } else {
            (0, 0)
        };

        // A residual with nothing in it has no placement to decide, so only one canonical home is
        // kept for it: otherwise the same layout would be produced several times. `Basic` is the one
        // that is viable for an empty residual whatever the routing.
        if full_tail == 0 && tails.full != TailHome::Basic {
            return None;
        }
        if hi_tail == 0 && tails.hi != TailHome::Basic {
            return None;
        }

        layout.full_to_add = full_whole;
        if routing.hi_add == HiAddRoute::AddHi {
            layout.hi_to_add_hi = hi_whole;
        } else {
            layout.hi_to_add = hi_whole;
        }

        // Room the Binary instances have already paid for and is still empty.
        let mut free_basic =
            basic.instances_and_cost(basic_ops).0 * basic.ops_per_instance - basic_ops;

        // Place the full-shape residual first only if that is what its home says; the ranking below
        // is what decides which residual gets the scarce free room, so no priority is baked in here.
        match tails.full {
            TailHome::Basic => {
                if full_tail > free_basic {
                    return None;
                }
                free_basic -= full_tail;
                basic_ops += full_tail;
            }
            TailHome::Add => layout.full_to_add += full_tail,
            TailHome::AddHi => return None,
        }

        // A `BinaryAdd` instance holding the full-shape residual has room to spare, and it can prove
        // low-limb additions too, so the other residual may ride there instead of paying for an
        // instance of its own.
        let free_add = {
            let ops = full_whole + if tails.full == TailHome::Add { full_tail } else { 0 };
            let instances = ops.div_ceil(add.ops_per_instance);
            instances * add.ops_per_instance - ops
        };

        match tails.hi {
            TailHome::Basic => {
                if hi_tail > free_basic {
                    return None;
                }
                basic_ops += hi_tail;
            }
            TailHome::Add => {
                // `BinaryAdd` would then be the last air to see the low-limb shape, so it must also
                // be the last to see the full one: a single instance cannot be told to account for
                // one shape's trailing frops but not the other's.
                if layout.full_to_add != totals.add {
                    return None;
                }
                if hi_tail > free_add {
                    return None;
                }
                layout.hi_to_add += hi_tail;
            }
            TailHome::AddHi => {
                if routing.hi_add != HiAddRoute::AddHi {
                    return None;
                }
                layout.hi_to_add_hi += hi_tail;
            }
        }

        layout.basic_ops = basic_ops;
        layout.add_ops = layout.full_to_add + layout.hi_to_add;
        layout.add_hi_ops = layout.hi_to_add_hi;

        match routing.ext {
            ExtRoute::Split => {
                layout.extension_ops = totals.extension;
                layout.extension_full_ops = totals.extension_full;
            }
            ExtRoute::FullOnly => {
                layout.extension_full_ops = totals.extension + totals.extension_full;
            }
        }

        for (budget, ops) in [
            (basic, layout.basic_ops),
            (add, layout.add_ops),
            (add_hi, layout.add_hi_ops),
            (Self::extension_budget(), layout.extension_ops),
            (Self::extension_full_budget(), layout.extension_full_ops),
        ] {
            let (instances, cost) = budget.instances_and_cost(ops);
            layout.instances += instances;
            layout.cost += cost;
        }

        Some(layout)
    }

    /// Picks the layout to use: cheapest first, then the one with fewest instances, then the most
    /// specific one (which is the order the candidates are yielded in).
    ///
    /// Ranking by cost is what makes "save the most expensive instance" dynamic: whichever residual
    /// would cost more to give an instance of its own is the one that ends up in the free room,
    /// without that order being hardcoded anywhere.
    fn best_layout(totals: &Totals) -> (Routing, BinaryLayout) {
        Routing::all()
            .flat_map(|routing| {
                TailHomes::all().filter_map(move |tails| {
                    Self::layout_of(totals, &routing, &tails).map(|layout| (routing, layout))
                })
            })
            .min_by_key(|(_, layout)| (layout.cost, layout.instances))
            .expect("at least one layout is always viable")
    }

    /// Per-chunk counts of each add shape, in chunk order.
    fn add_counts(counters: &[(ChunkId, &BinaryCounter)]) -> (Vec<u64>, Vec<u64>) {
        (
            counters.iter().map(|(_, c)| c.counter_add_hi.inst_count).collect(),
            counters.iter().map(|(_, c)| c.counter_add.inst_count).collect(),
        )
    }

    /// How one air must filter a shape in one chunk.
    ///
    /// `before` is what the airs ahead of it in the chain take from the chunk and `mine` is its own
    /// share. An air that takes nothing drops the shape whole, which is also what keeps it from
    /// accounting for that chunk's frops — unless it is the designated owner of them, in which case
    /// it has to keep accepting the shape so it can see them.
    fn drop_for(before: u64, mine: u64, owns_frops: bool) -> ShapeDrop {
        if mine == 0 && !owns_frops {
            ShapeDrop::all()
        } else if before == 0 {
            ShapeDrop::none()
        } else {
            ShapeDrop::first(before)
        }
    }

    /// Whether an air accounts for a chunk's frops of a shape.
    ///
    /// The air that sees the end of the shape in that chunk is the one that can catch the frops
    /// trailing after its last operation, so ownership goes to whoever takes the final share. When
    /// the chunk holds no operation of the shape at all there is no such air, so it falls to the head
    /// of the chain — `is_head` is what keeps the other airs from claiming it too.
    fn owns_frops(is_head: bool, before: u64, mine: u64, total: u64) -> bool {
        if total == 0 {
            is_head
        } else {
            mine > 0 && before + mine == total
        }
    }

    /// Plans the instances of one add-capable air: `Binary` or `BinaryAdd`.
    ///
    /// `own_basic` marks the `Binary` air, which also proves the basic operations. `hi_before` and
    /// `full_before` are what the airs ahead of it take from each chunk, and `hi_mine` / `full_mine`
    /// its own shares.
    #[allow(clippy::too_many_arguments)]
    fn plan_add_capable(
        &self,
        counters: &[(ChunkId, &BinaryCounter)],
        own_basic: bool,
        hi_counts: &[u64],
        full_counts: &[u64],
        hi_before: &[u64],
        hi_mine: &[u64],
        hi_is_head: bool,
        full_before: &[u64],
        full_mine: &[u64],
        full_is_head: bool,
        budget: AirBudget,
        airgroup_id: usize,
        air_id: usize,
    ) -> Vec<Plan> {
        let counts: Vec<InstFropsCount> = counters
            .iter()
            .enumerate()
            .map(|(i, (chunk_id, c))| {
                let mut inst = hi_mine[i] + full_mine[i];
                let mut frops = 0;

                if own_basic {
                    inst += c.counter_basic_wo_add.inst_count;
                    frops += c.counter_basic_wo_add.frops_count;
                }
                if Self::owns_frops(hi_is_head, hi_before[i], hi_mine[i], hi_counts[i]) {
                    frops += c.counter_add_hi.frops_count;
                }
                if Self::owns_frops(full_is_head, full_before[i], full_mine[i], full_counts[i]) {
                    frops += c.counter_add.frops_count;
                }

                InstFropsCount::new(*chunk_id, inst, frops)
            })
            .collect();

        plan_with_frops(&counts, budget.ops_per_instance)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let collect_info: HashMap<ChunkId, BinaryCollectInfo> = collect_info
                    .into_iter()
                    .map(|(chunk_id, (count, force_execute_to_end, skipper))| {
                        let i = chunk_id.0;
                        (
                            chunk_id,
                            BinaryCollectInfo {
                                count,
                                skipper,
                                hi_drop: Self::drop_for(
                                    hi_before[i],
                                    hi_mine[i],
                                    Self::owns_frops(
                                        hi_is_head,
                                        hi_before[i],
                                        hi_mine[i],
                                        hi_counts[i],
                                    ),
                                ),
                                full_drop: Self::drop_for(
                                    full_before[i],
                                    full_mine[i],
                                    Self::owns_frops(
                                        full_is_head,
                                        full_before[i],
                                        full_mine[i],
                                        full_counts[i],
                                    ),
                                ),
                                force_execute_to_end,
                            },
                        )
                    })
                    .collect();

                let meta: Box<dyn Any + Send + Sync> = Box::new(collect_info);
                Plan::new(
                    airgroup_id,
                    air_id,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(meta),
                )
            })
            .collect()
    }

    /// Plans the packed `BinaryAddHi` instances, which take a prefix of the low-limb shape.
    ///
    /// Its capacity is measured in operations rather than rows, since every row proves
    /// [`ADDS_X_ROW`] of them.
    fn plan_for_add_hi(
        &self,
        counters: &[(ChunkId, &BinaryCounter)],
        hi_counts: &[u64],
        hi_mine: &[u64],
    ) -> Vec<Plan> {
        let counts: Vec<InstFropsCount> = counters
            .iter()
            .enumerate()
            .map(|(i, (chunk_id, c))| {
                let frops = if Self::owns_frops(true, 0, hi_mine[i], hi_counts[i]) {
                    c.counter_add_hi.frops_count
                } else {
                    0
                };
                InstFropsCount::new(*chunk_id, hi_mine[i], frops)
            })
            .collect();

        plan_with_frops(&counts, Self::add_hi_budget().ops_per_instance)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let meta: Box<dyn Any + Send + Sync> = Box::new(collect_info);
                Plan::new(
                    BinaryAddHiTrace::<()>::AIRGROUP_ID,
                    BinaryAddHiTrace::<()>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(meta),
                )
            })
            .collect()
    }

    /// Plans the extension instances of one scope.
    fn plan_for_extensions(
        &self,
        counters: &[(ChunkId, &BinaryCounter)],
        scope: ExtensionScope,
    ) -> Vec<Plan> {
        let counts: Vec<InstFropsCount> = counters
            .iter()
            .map(|(chunk_id, c)| {
                let (inst, frops) = match scope {
                    ExtensionScope::Clean => {
                        (c.counter_extension.inst_count, c.counter_extension.frops_count)
                    }
                    ExtensionScope::Dirty => {
                        (c.counter_extension_full.inst_count, c.counter_extension_full.frops_count)
                    }
                    ExtensionScope::All => (
                        c.counter_extension.inst_count + c.counter_extension_full.inst_count,
                        c.counter_extension.frops_count + c.counter_extension_full.frops_count,
                    ),
                };
                InstFropsCount::new(*chunk_id, inst, frops)
            })
            .collect();

        let (airgroup_id, air_id, budget) = if scope == ExtensionScope::Clean {
            (
                BinaryExtensionTrace::<()>::AIRGROUP_ID,
                BinaryExtensionTrace::<()>::AIR_ID,
                Self::extension_budget(),
            )
        } else {
            (
                BinaryExtensionFullTrace::<()>::AIRGROUP_ID,
                BinaryExtensionFullTrace::<()>::AIR_ID,
                Self::extension_full_budget(),
            )
        };

        plan_with_frops(&counts, budget.ops_per_instance)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let meta: Box<dyn Any + Send + Sync> = Box::new((scope, collect_info));
                Plan::new(
                    airgroup_id,
                    air_id,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(meta),
                )
            })
            .collect()
    }
}

impl<F: PrimeField64> Planner for BinaryPlanner<F> {
    /// Generates execution plans for binary instances and tables.
    ///
    /// # Arguments
    /// * `counters` - A vector of counters, each associated with a `ChunkId` and `BinaryCounter`
    ///   metrics data.
    ///
    /// # Returns
    /// A vector of `Plan` instances representing execution configurations for the instances and
    /// tables.
    ///
    /// # Panics
    /// Panics if any counter cannot be downcasted to an `BinaryCounter`.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let mut totals = Totals::default();

        let binary_counters: Vec<(ChunkId, &BinaryCounter)> = counters
            .iter()
            .map(|(chunk_id, counter)| {
                let counter = Metrics::as_any(&**counter).downcast_ref::<BinaryCounter>().unwrap();
                totals.basic_wo_add += counter.counter_basic_wo_add.inst_count;
                totals.add += counter.counter_add.inst_count;
                totals.add_hi += counter.counter_add_hi.inst_count;
                totals.extension += counter.counter_extension.inst_count;
                totals.extension_full += counter.counter_extension_full.inst_count;
                (*chunk_id, counter)
            })
            .collect();

        let (routing, layout) = Self::best_layout(&totals);

        tracing::debug!(
            "··· Binary routing {:?}: {} instances, cost {} \
             (ops basic={} add={} add_hi={} ext={} ext_full={}, add_hi packs {} rows)",
            routing,
            layout.instances,
            layout.cost,
            layout.basic_ops,
            layout.add_ops,
            layout.add_hi_ops,
            layout.extension_ops,
            layout.extension_full_ops,
            rows_needed(layout.add_hi_ops),
        );

        // Per-chunk shares of each shape, following the chain of airs that own it:
        //   low-limb : BinaryAddHi -> BinaryAdd -> Binary
        //   full     : BinaryAdd -> Binary
        // Each air takes a contiguous range, so a plain skip separates one from the next.
        let (hi_counts, full_counts) = Self::add_counts(&binary_counters);

        let hi_add_hi = Self::dedicated_share(&hi_counts, layout.hi_to_add_hi);
        let hi_through_add =
            Self::dedicated_share(&hi_counts, layout.hi_to_add_hi + layout.hi_to_add);
        let hi_add: Vec<u64> = hi_through_add.iter().zip(&hi_add_hi).map(|(t, p)| t - p).collect();
        let hi_basic: Vec<u64> =
            hi_counts.iter().zip(&hi_through_add).map(|(n, t)| n - t).collect();

        let full_add = Self::dedicated_share(&full_counts, layout.full_to_add);
        let full_basic: Vec<u64> = full_counts.iter().zip(&full_add).map(|(n, t)| n - t).collect();

        let mut plans = Vec::new();

        // Extensions. Dirty operands can only be proven by the full air, so a Split routing only
        // creates the instances each side actually needs.
        match routing.ext {
            ExtRoute::Split => {
                if totals.extension > 0 {
                    plans.append(
                        &mut self.plan_for_extensions(&binary_counters, ExtensionScope::Clean),
                    );
                }
                if totals.extension_full > 0 {
                    plans.append(
                        &mut self.plan_for_extensions(&binary_counters, ExtensionScope::Dirty),
                    );
                }
            }
            ExtRoute::FullOnly => {
                if layout.extension_full_ops > 0 {
                    plans.append(
                        &mut self.plan_for_extensions(&binary_counters, ExtensionScope::All),
                    );
                }
            }
        }

        // Packed adds: the prefix of the low-limb shape that fills whole instances.
        if layout.add_hi_ops > 0 {
            plans.append(&mut self.plan_for_add_hi(&binary_counters, &hi_counts, &hi_add_hi));
        }

        // Dedicated adds: the full-shape prefix, plus any residual riding along.
        if layout.add_ops > 0 {
            plans.append(&mut self.plan_add_capable(
                &binary_counters,
                false,
                &hi_counts,
                &full_counts,
                &hi_add_hi,
                &hi_add,
                routing.hi_add == HiAddRoute::Add,
                &vec![0; full_counts.len()],
                &full_add,
                routing.full_add == FullAddRoute::Add,
                Self::add_budget(),
                BinaryAddTrace::<()>::AIRGROUP_ID,
                BinaryAddTrace::<()>::AIR_ID,
            ));
        }

        // Basic ops, plus the additions that ride in the room these instances already paid for.
        // Always planned so the basic operations of every chunk are covered.
        plans.append(&mut self.plan_add_capable(
            &binary_counters,
            true,
            &hi_counts,
            &full_counts,
            &hi_through_add,
            &hi_basic,
            routing.hi_add == HiAddRoute::Basic,
            &full_add,
            &full_basic,
            routing.full_add == FullAddRoute::Basic,
            Self::basic_budget(),
            BinaryTrace::<()>::AIRGROUP_ID,
            BinaryTrace::<()>::AIR_ID,
        ));

        plans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_pil::{
        BinaryAddHiTraceRow, BinaryAddTraceRow, BinaryExtensionFullTraceRow,
        BinaryExtensionTraceRow, BinaryTraceRow,
    };

    type TestPlanner = BinaryPlanner<fields::Goldilocks>;

    /// The declared column counts must stay in step with the trace geometry they stand for, and the
    /// cost of an instance must be exactly those columns times that air's own rows.
    #[test]
    fn columns_match_the_traces() {
        type F = fields::Goldilocks;

        for (name, declared, row_size, num_rows, budget) in [
            (
                "Binary",
                columns::BINARY,
                BinaryTrace::<BinaryTraceRow<F>>::ROW_SIZE,
                BinaryTrace::<()>::NUM_ROWS,
                TestPlanner::basic_budget(),
            ),
            (
                "BinaryAdd",
                columns::BINARY_ADD,
                BinaryAddTrace::<BinaryAddTraceRow<F>>::ROW_SIZE,
                BinaryAddTrace::<()>::NUM_ROWS,
                TestPlanner::add_budget(),
            ),
            (
                "BinaryAddHi",
                columns::BINARY_ADD_HI,
                BinaryAddHiTrace::<BinaryAddHiTraceRow<F>>::ROW_SIZE,
                BinaryAddHiTrace::<()>::NUM_ROWS,
                TestPlanner::add_hi_budget(),
            ),
            (
                "BinaryExtension",
                columns::BINARY_EXTENSION,
                BinaryExtensionTrace::<BinaryExtensionTraceRow<F>>::ROW_SIZE,
                BinaryExtensionTrace::<()>::NUM_ROWS,
                TestPlanner::extension_budget(),
            ),
            (
                "BinaryExtensionFull",
                columns::BINARY_EXTENSION_FULL,
                BinaryExtensionFullTrace::<BinaryExtensionFullTraceRow<F>>::ROW_SIZE,
                BinaryExtensionFullTrace::<()>::NUM_ROWS,
                TestPlanner::extension_full_budget(),
            ),
        ] {
            assert_eq!(declared, row_size as u64, "declared columns of {name} are stale");
            assert_eq!(
                budget.cost_per_instance,
                declared * num_rows as u64,
                "cost of a {name} instance must be its columns times its rows"
            );
        }
    }

    /// Capacity follows each air's own rows, and the packed one holds ADDS_X_ROW per row.
    #[test]
    fn capacity_follows_the_rows_of_each_air() {
        assert_eq!(
            TestPlanner::add_budget().ops_per_instance,
            BinaryAddTrace::<()>::NUM_ROWS as u64
        );
        assert_eq!(
            TestPlanner::add_hi_budget().ops_per_instance,
            ADDS_X_ROW as u64 * BinaryAddHiTrace::<()>::NUM_ROWS as u64
        );
    }

    /// With nothing to prove nothing is planned.
    #[test]
    fn empty_totals_need_no_instances() {
        let (_, layout) = TestPlanner::best_layout(&Totals::default());
        assert_eq!(layout.instances, 0);
        assert_eq!(layout.cost, 0);
    }

    /// Additions ride in the leftover room of the `Binary` instances while there is any, and only
    /// move to their dedicated airs once that room is gone. This is what the residual placement
    /// exists for: filling the tail of a `Binary` instance must never cost an extra dedicated
    /// instance.
    #[test]
    fn additions_fill_the_binary_leftover_before_paying_for_an_instance() {
        let capacity = TestPlanner::basic_budget().ops_per_instance;

        let roomy = Totals { basic_wo_add: 10, add: 10, add_hi: 10, ..Default::default() };
        let (_, layout) = TestPlanner::best_layout(&roomy);
        assert_eq!(layout.instances, 1, "a single Binary instance must absorb everything");
        assert_eq!(layout.add_ops, 0);
        assert_eq!(layout.add_hi_ops, 0);
        assert_eq!(layout.basic_ops, 30);

        // Basic ops fill the instance exactly: no room left, so the additions pay for their own.
        let full = Totals { basic_wo_add: capacity, add: 10, add_hi: 10, ..Default::default() };
        let (_, layout) = TestPlanner::best_layout(&full);
        assert_eq!(layout.basic_ops, capacity, "no room was left to spill into");
        assert!(layout.add_ops + layout.add_hi_ops > 0, "additions must go to a dedicated air");
    }

    /// Only the residual moves: the dedicated air keeps its whole instances and one is saved.
    #[test]
    fn residual_placement_saves_the_partial_dedicated_instance() {
        let add = TestPlanner::add_budget();

        let totals =
            Totals { basic_wo_add: 10, add: 2 * add.ops_per_instance + 5, ..Default::default() };
        let (_, layout) = TestPlanner::best_layout(&totals);

        assert_eq!(layout.full_to_add, 2 * add.ops_per_instance, "whole instances stay dedicated");
        assert_eq!(layout.basic_ops, 15, "the residual rides with the basic operations");
        assert_eq!(layout.add_ops % add.ops_per_instance, 0, "no partial dedicated instance");
        // 1 Binary + 2 BinaryAdd, instead of 1 Binary + 3 BinaryAdd.
        assert_eq!(layout.instances, 3);
    }

    /// The low-limb residual can ride in the room of a `BinaryAdd` instance that the full-shape
    /// residual is already paying for, saving the packed instance it would need otherwise. This is
    /// the case that `Binary`'s room alone cannot cover.
    #[test]
    fn low_limb_residual_rides_in_the_add_instance_room() {
        let add = TestPlanner::add_budget();
        let add_hi = TestPlanner::add_hi_budget();
        let basic = TestPlanner::basic_budget();

        // Binary is exactly full, so it has no room at all; the full-shape residual needs its own
        // BinaryAdd instance, which then has plenty of room for the low-limb residual.
        let totals = Totals {
            basic_wo_add: basic.ops_per_instance,
            add: 5,
            add_hi: add_hi.ops_per_instance + 7,
            ..Default::default()
        };
        let (_, layout) = TestPlanner::best_layout(&totals);

        assert_eq!(layout.hi_to_add_hi, add_hi.ops_per_instance, "whole packed instance stays");
        assert_eq!(layout.hi_to_add, 7, "the low-limb residual rides in the BinaryAdd instance");
        assert_eq!(layout.full_to_add, 5);
        assert_eq!(layout.add_ops, 12);
        // 1 Binary + 1 BinaryAdd + 1 BinaryAddHi, instead of a second packed instance.
        assert_eq!(layout.instances, 3);
        assert_eq!(add.instances_and_cost(layout.add_ops).0, 1);
    }

    /// Which residual gets the scarce free room is decided by cost, not by a fixed order: the one
    /// whose own instance would be more expensive is the one that is saved.
    #[test]
    fn the_more_expensive_instance_is_the_one_saved() {
        let add = TestPlanner::add_budget();
        let add_hi = TestPlanner::add_hi_budget();
        assert!(
            add_hi.cost_per_instance > add.cost_per_instance,
            "this test assumes the packed instance is the pricier one"
        );

        // Room for exactly one of the two residuals.
        let totals = Totals {
            basic_wo_add: TestPlanner::basic_budget().ops_per_instance - 5,
            add: add.ops_per_instance + 5,
            add_hi: add_hi.ops_per_instance + 5,
            ..Default::default()
        };
        let (_, layout) = TestPlanner::best_layout(&totals);

        // The packed residual is the one that must be spared its own instance.
        assert_eq!(layout.hi_to_add_hi, add_hi.ops_per_instance);
        assert_eq!(layout.add_hi_ops % add_hi.ops_per_instance, 0, "no partial packed instance");
    }

    /// The packed air only beats `BinaryAdd` past one plain instance: it is cheaper per operation
    /// but its rows are wider.
    #[test]
    fn packed_air_wins_past_one_plain_instance() {
        let capacity = TestPlanner::add_budget().ops_per_instance;

        let (routing, _) =
            TestPlanner::best_layout(&Totals { add_hi: capacity, ..Default::default() });
        assert_eq!(routing.hi_add, HiAddRoute::Add);

        let (routing, _) =
            TestPlanner::best_layout(&Totals { add_hi: capacity + 1, ..Default::default() });
        assert_eq!(routing.hi_add, HiAddRoute::AddHi);
    }

    /// Extension operations that all need the full air must not create a reduced instance.
    #[test]
    fn no_reduced_extension_instance_when_all_operands_are_dirty() {
        let totals = Totals { extension_full: 1_000_000, ..Default::default() };
        let (_, layout) = TestPlanner::best_layout(&totals);
        assert_eq!(layout.extension_ops, 0);
        assert_eq!(layout.extension_full_ops, 1_000_000);
    }

    /// A single clean extension operation alongside many dirty ones is cheaper riding in the full
    /// air than paying for a whole reduced instance of its own.
    #[test]
    fn a_lone_clean_extension_op_rides_with_the_dirty_ones() {
        let totals = Totals { extension: 1, extension_full: 1_000_000, ..Default::default() };
        let (routing, layout) = TestPlanner::best_layout(&totals);
        assert_eq!(routing.ext, ExtRoute::FullOnly);
        assert_eq!(layout.instances, 1);
    }

    /// Cost, not instance count alone, decides: the same number of instances of different airs are
    /// not equally expensive.
    #[test]
    fn cost_distinguishes_airs_with_equal_instance_counts() {
        let basic = TestPlanner::basic_budget();
        let add = TestPlanner::add_budget();
        assert_eq!(basic.instances_and_cost(1).0, add.instances_and_cost(1).0);
        assert!(
            basic.instances_and_cost(1).1 > add.instances_and_cost(1).1,
            "a Binary instance must cost more than a BinaryAdd one"
        );
    }

    /// Every operation must end up in exactly one air, whatever the routing and placement: the
    /// operations the airs prove have to add up to the operations that were counted, and the shares
    /// of each shape must tile it.
    #[test]
    fn every_layout_conserves_the_operations() {
        let capacity = TestPlanner::basic_budget().ops_per_instance;
        let candidates = [0, 1, 7, capacity - 1, capacity, capacity + 1, 3 * capacity + 11];

        for &basic_wo_add in &candidates {
            for &add in &candidates {
                for &add_hi in &candidates {
                    let totals =
                        Totals { basic_wo_add, add, add_hi, extension: 13, extension_full: 17 };
                    for routing in Routing::all() {
                        for tails in TailHomes::all() {
                            let Some(layout) = TestPlanner::layout_of(&totals, &routing, &tails)
                            else {
                                continue;
                            };
                            assert_eq!(
                                layout.basic_ops + layout.add_ops + layout.add_hi_ops,
                                basic_wo_add + add + add_hi,
                                "operations lost or duplicated for {routing:?}/{tails:?}"
                            );
                            assert_eq!(
                                layout.extension_ops + layout.extension_full_ops,
                                totals.extension + totals.extension_full
                            );
                            // The shares of each shape must address operations that exist.
                            assert!(layout.full_to_add <= add);
                            assert!(layout.hi_to_add_hi + layout.hi_to_add <= add_hi);
                            // The residual must never have grown the Binary instance count.
                            assert!(layout.basic_ops >= basic_wo_add);
                        }
                    }
                }
            }
        }
    }

    /// Placing a residual must never cost a `Binary` instance: it only ever uses room that the basic
    /// operations already paid for.
    #[test]
    fn residual_placement_never_adds_a_binary_instance() {
        let basic = TestPlanner::basic_budget();
        let candidates = [0, 1, 7, basic.ops_per_instance - 1, basic.ops_per_instance + 3];

        for &basic_wo_add in &candidates {
            for &add in &candidates {
                for &add_hi in &candidates {
                    let totals = Totals { basic_wo_add, add, add_hi, ..Default::default() };
                    let (routing, layout) = TestPlanner::best_layout(&totals);

                    // The instances forced by the operations that can only live in Binary.
                    let forced = basic_wo_add
                        + if routing.full_add == FullAddRoute::Basic { add } else { 0 }
                        + if routing.hi_add == HiAddRoute::Basic { add_hi } else { 0 };
                    assert_eq!(
                        basic.instances_and_cost(layout.basic_ops).0,
                        basic.instances_and_cost(forced).0,
                        "residual placement grew the Binary instance count on {totals:?}"
                    );
                }
            }
        }
    }

    /// The per-chunk split of a shape hands the leading air a prefix, in chunk order.
    #[test]
    fn dedicated_share_splits_in_chunk_order() {
        let counts = [5u64, 0, 3, 7];

        assert_eq!(TestPlanner::dedicated_share(&counts, 0), vec![0, 0, 0, 0]);
        assert_eq!(TestPlanner::dedicated_share(&counts, 5), vec![5, 0, 0, 0]);
        assert_eq!(TestPlanner::dedicated_share(&counts, 7), vec![5, 0, 2, 0]);
        assert_eq!(TestPlanner::dedicated_share(&counts, 15), vec![5, 0, 3, 7]);
        assert_eq!(TestPlanner::dedicated_share(&counts, 100), vec![5, 0, 3, 7]);
    }

    /// An air that takes nothing of a shape drops it whole; one that takes it from the start takes
    /// everything; one that takes a later range skips what the airs before it took.
    #[test]
    fn drops_describe_each_air_share() {
        assert_eq!(TestPlanner::drop_for(0, 0, false), ShapeDrop::all());
        assert_eq!(TestPlanner::drop_for(4, 0, false), ShapeDrop::all());
        assert_eq!(TestPlanner::drop_for(0, 4, false), ShapeDrop::none());
        assert_eq!(TestPlanner::drop_for(3, 1, false), ShapeDrop::first(3));
        // The designated frops owner of a chunk with no operation of the shape must still accept it.
        assert_eq!(TestPlanner::drop_for(0, 0, true), ShapeDrop::none());
    }

    /// Exactly one air accounts for a chunk's frops of a shape: the one that sees the shape's end
    /// there, or the head of the chain when the chunk holds no operation of it.
    #[test]
    fn exactly_one_air_owns_each_chunk_frops() {
        // A chunk whose 4 operations are split 3 + 1 between two airs.
        assert!(!TestPlanner::owns_frops(true, 0, 3, 4), "the leading air does not see the end");
        assert!(TestPlanner::owns_frops(false, 3, 1, 4), "the trailing air does");

        // A chunk wholly owned by the leading air.
        assert!(TestPlanner::owns_frops(true, 0, 4, 4));
        assert!(!TestPlanner::owns_frops(false, 4, 0, 4));

        // A chunk with frops but no operation of the shape: only the head of the chain owns them.
        assert!(TestPlanner::owns_frops(true, 0, 0, 0));
        assert!(!TestPlanner::owns_frops(false, 0, 0, 0));

        // Over a three-air chain, whatever the split, exactly one owns them.
        for total in 0..6u64 {
            for first in 0..=total {
                for second in 0..=(total - first) {
                    let third = total - first - second;
                    let owners = [
                        TestPlanner::owns_frops(true, 0, first, total),
                        TestPlanner::owns_frops(false, first, second, total),
                        TestPlanner::owns_frops(false, first + second, third, total),
                    ]
                    .iter()
                    .filter(|owns| **owns)
                    .count();
                    assert_eq!(
                        owners, 1,
                        "total={total} split=({first},{second},{third}) has {owners} owners"
                    );
                }
            }
        }
    }
}
