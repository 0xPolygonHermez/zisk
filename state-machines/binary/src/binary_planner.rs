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

use crate::{rows_needed, AddScope, BinaryCounter, ExtensionScope, ADDS_X_ROW};
use fields::PrimeField64;
use std::any::Any;
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

/// Rows and instances each binary air needs under a given routing.
#[derive(Default, Clone, Copy, Debug)]
struct BinaryLayout {
    basic_ops: u64,
    add_ops: u64,
    add_hi_ops: u64,
    extension_ops: u64,
    extension_full_ops: u64,

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

    /// Add shapes the `Binary` air is responsible for under this routing.
    fn basic_add_scope(&self) -> AddScope {
        AddScope {
            full: self.full_add == FullAddRoute::Basic,
            hi: self.hi_add == HiAddRoute::Basic,
        }
    }

    /// Add shapes the `BinaryAdd` air is responsible for under this routing.
    fn add_scope(&self) -> AddScope {
        AddScope { full: self.full_add == FullAddRoute::Add, hi: self.hi_add == HiAddRoute::Add }
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

    /// Operations, instances and cost of each air under `routing`.
    fn layout_of(totals: &Totals, routing: &Routing) -> BinaryLayout {
        let mut layout = BinaryLayout { basic_ops: totals.basic_wo_add, ..Default::default() };

        match routing.full_add {
            FullAddRoute::Basic => layout.basic_ops += totals.add,
            FullAddRoute::Add => layout.add_ops += totals.add,
        }
        match routing.hi_add {
            HiAddRoute::Basic => layout.basic_ops += totals.add_hi,
            HiAddRoute::Add => layout.add_ops += totals.add_hi,
            HiAddRoute::AddHi => layout.add_hi_ops = totals.add_hi,
        }

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
            (Self::basic_budget(), layout.basic_ops),
            (Self::add_budget(), layout.add_ops),
            (Self::add_hi_budget(), layout.add_hi_ops),
            (Self::extension_budget(), layout.extension_ops),
            (Self::extension_full_budget(), layout.extension_full_ops),
        ] {
            let (instances, cost) = budget.instances_and_cost(ops);
            layout.instances += instances;
            layout.cost += cost;
        }

        layout
    }

    /// Picks the routing to use: cheapest first, then the one with fewest instances, then the most
    /// specific one (which is the order [`Routing::all`] yields them in).
    fn best_routing(totals: &Totals) -> (Routing, BinaryLayout) {
        Routing::all()
            .map(|routing| {
                let layout = Self::layout_of(totals, &routing);
                (routing, layout)
            })
            .min_by_key(|(_, layout)| (layout.cost, layout.instances))
            .expect("Routing::all() is never empty")
    }

    /// Plans the `Binary` instances, which also take the additions routed to them.
    fn plan_for_basics(
        &self,
        counters: &[(ChunkId, &BinaryCounter)],
        add_scope: AddScope,
    ) -> Vec<Plan> {
        let counts: Vec<InstFropsCount> = counters
            .iter()
            .map(|(chunk_id, c)| {
                let mut inst = c.counter_basic_wo_add.inst_count;
                let mut frops = c.counter_basic_wo_add.frops_count;
                if add_scope.full {
                    inst += c.counter_add.inst_count;
                    frops += c.counter_add.frops_count;
                }
                if add_scope.hi {
                    inst += c.counter_add_hi.inst_count;
                    frops += c.counter_add_hi.frops_count;
                }
                InstFropsCount::new(*chunk_id, inst, frops)
            })
            .collect();

        plan_with_frops(&counts, Self::basic_budget().ops_per_instance)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let meta: Box<dyn Any + Send + Sync> = Box::new((add_scope, collect_info));
                Plan::new(
                    BinaryTrace::<()>::AIRGROUP_ID,
                    BinaryTrace::<()>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(meta),
                )
            })
            .collect()
    }

    /// Plans the dedicated `BinaryAdd` instances for the add shapes routed to them.
    fn plan_for_adds(
        &self,
        counters: &[(ChunkId, &BinaryCounter)],
        add_scope: AddScope,
    ) -> Vec<Plan> {
        if add_scope.is_empty() {
            return vec![];
        }

        let counts: Vec<InstFropsCount> = counters
            .iter()
            .map(|(chunk_id, c)| {
                let mut inst = 0;
                let mut frops = 0;
                if add_scope.full {
                    inst += c.counter_add.inst_count;
                    frops += c.counter_add.frops_count;
                }
                if add_scope.hi {
                    inst += c.counter_add_hi.inst_count;
                    frops += c.counter_add_hi.frops_count;
                }
                InstFropsCount::new(*chunk_id, inst, frops)
            })
            .collect();

        plan_with_frops(&counts, Self::add_budget().ops_per_instance)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let meta: Box<dyn Any + Send + Sync> = Box::new((add_scope, collect_info));
                Plan::new(
                    BinaryAddTrace::<()>::AIRGROUP_ID,
                    BinaryAddTrace::<()>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(meta),
                )
            })
            .collect()
    }

    /// Plans the packed `BinaryAddHi` instances.
    ///
    /// Its capacity is measured in operations rather than rows, since every row proves
    /// [`ADDS_X_ROW`] of them.
    fn plan_for_add_hi(&self, counters: &[(ChunkId, &BinaryCounter)]) -> Vec<Plan> {
        let counts: Vec<InstFropsCount> = counters
            .iter()
            .map(|(chunk_id, c)| {
                InstFropsCount::new(
                    *chunk_id,
                    c.counter_add_hi.inst_count,
                    c.counter_add_hi.frops_count,
                )
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

        let (routing, layout) = Self::best_routing(&totals);

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

        // Packed adds.
        if layout.add_hi_ops > 0 {
            plans.append(&mut self.plan_for_add_hi(&binary_counters));
        }

        // Dedicated adds.
        if layout.add_ops > 0 {
            plans.append(&mut self.plan_for_adds(&binary_counters, routing.add_scope()));
        }

        // Basic ops, plus whatever additions were routed here. Always planned so the basic
        // operations of every chunk are covered.
        plans.append(&mut self.plan_for_basics(&binary_counters, routing.basic_add_scope()));

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

    /// With nothing to prove nothing is planned, and the routing is the most specific one.
    #[test]
    fn empty_totals_need_no_instances() {
        let (routing, layout) = TestPlanner::best_routing(&Totals::default());
        assert_eq!(layout.instances, 0);
        assert_eq!(layout.cost, 0);
        assert_eq!(routing.hi_add, HiAddRoute::AddHi);
        assert_eq!(routing.full_add, FullAddRoute::Add);
    }

    /// Operations ride along in an air that is needed anyway rather than paying for an instance of
    /// their own: with a handful of each kind the whole load fits in one add instance and one
    /// extension instance, so no separate packed or reduced instance is created.
    #[test]
    fn small_loads_ride_along_instead_of_paying_for_an_instance() {
        let totals =
            Totals { add: 10, add_hi: 10, extension: 10, extension_full: 10, ..Default::default() };
        let (routing, layout) = TestPlanner::best_routing(&totals);

        // Both add shapes share the single dedicated add instance...
        assert_eq!(routing.full_add, FullAddRoute::Add);
        assert_eq!(routing.hi_add, HiAddRoute::Add);
        // ...and the clean extension ops ride in the full air, which is needed for the dirty ones.
        assert_eq!(routing.ext, ExtRoute::FullOnly);
        assert_eq!(layout.instances, 2);
    }

    /// The packed air is cheaper per operation (it fits ADDS_X_ROW of them per row) but its rows are
    /// wider, so it only pays off once the additions no longer fit in a single plain instance. This
    /// pins that crossover.
    #[test]
    fn packed_air_wins_past_one_plain_instance() {
        let capacity = TestPlanner::add_budget().ops_per_instance;

        // Up to one plain instance, the narrower BinaryAdd row is the cheaper home.
        let (routing, _) =
            TestPlanner::best_routing(&Totals { add_hi: capacity, ..Default::default() });
        assert_eq!(routing.hi_add, HiAddRoute::Add);

        // Past it, a second plain instance costs more than a single packed one.
        let (routing, _) =
            TestPlanner::best_routing(&Totals { add_hi: capacity + 1, ..Default::default() });
        assert_eq!(routing.hi_add, HiAddRoute::AddHi);
    }

    /// Packing wins as soon as it saves instances: the same additions take ADDS_X_ROW times fewer
    /// rows in the packed air.
    #[test]
    fn packing_wins_when_it_saves_instances() {
        let per_instance = TestPlanner::add_budget().ops_per_instance;
        let totals = Totals { add_hi: ADDS_X_ROW as u64 * per_instance, ..Default::default() };

        let (routing, layout) = TestPlanner::best_routing(&totals);
        assert_eq!(routing.hi_add, HiAddRoute::AddHi);
        // One packed instance instead of ADDS_X_ROW plain ones.
        assert_eq!(layout.instances, 1);
    }

    /// Additions are only merged into the wide `Binary` air while its instances still have room;
    /// once the basic operations fill it, the dedicated airs are much cheaper than a second one.
    #[test]
    fn additions_leave_binary_once_it_is_full() {
        let capacity = TestPlanner::basic_budget().ops_per_instance;

        // Basic ops leave room: the additions ride along for free.
        let roomy = Totals { basic_wo_add: 10, add: 10, add_hi: 10, ..Default::default() };
        let (routing, layout) = TestPlanner::best_routing(&roomy);
        assert_eq!(routing.full_add, FullAddRoute::Basic);
        assert_eq!(routing.hi_add, HiAddRoute::Basic);
        assert_eq!(layout.instances, 1);

        // Basic ops fill the instance exactly: now every addition would force a second Binary
        // instance, so they move to their dedicated airs instead.
        let full = Totals { basic_wo_add: capacity, add: 10, add_hi: 10, ..Default::default() };
        let (routing, _) = TestPlanner::best_routing(&full);
        assert_ne!(routing.full_add, FullAddRoute::Basic);
        assert_ne!(routing.hi_add, HiAddRoute::Basic);
    }

    /// Extension operations that all need the full air must not create a reduced instance.
    #[test]
    fn no_reduced_extension_instance_when_all_operands_are_dirty() {
        let totals = Totals { extension_full: 1_000_000, ..Default::default() };
        let (_, layout) = TestPlanner::best_routing(&totals);
        assert_eq!(layout.extension_ops, 0);
        assert_eq!(layout.extension_full_ops, 1_000_000);
    }

    /// A single clean extension operation alongside many dirty ones is cheaper riding in the full
    /// air than paying for a whole reduced instance of its own.
    #[test]
    fn a_lone_clean_extension_op_rides_with_the_dirty_ones() {
        let totals = Totals { extension: 1, extension_full: 1_000_000, ..Default::default() };
        let (routing, layout) = TestPlanner::best_routing(&totals);
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
}
