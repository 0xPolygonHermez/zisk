//! Unit tests for the DMA air-selection strategy.
//! Declared from `dma_strategy.rs` via `#[cfg(test)] #[path = …] mod tests;`, so it stays a child
//! module of `dma_strategy` and keeps `super::` access to privates.

use super::*;
use crate::{DmaWithPrePostInput, DMA_WPP_CLASS_DOUBLE, DMA_WPP_CLASS_SINGLE};
use proofman_fields::Goldilocks;
use zisk_core::zisk_ops::ZiskOp;
use zisk_precomp_helpers::DmaInfo;

type Strategy = DmaStrategy<Goldilocks>;

/// Rows one instance of each 64-bit-aligned air holds, in [`air`] order.
fn caps() -> [usize; air::COUNT] {
    Strategy::dma_64_aligned_airs().map(|choice| choice.rows as usize)
}

/// Builds the counters slice the 64-bit-aligned strategy expects, indexed by `DMA_COUNTER_*`.
///
/// The packed counts default to an eighth of the plain ones, which is the ratio the packed airs
/// actually achieve (`op_x_row: 8`).
fn rows_of(
    memcpy: usize,
    memset: usize,
    memcmp: usize,
    inputcpy: usize,
) -> [usize; DMA_COUNTER_OPS_EXT] {
    let mut rows = [0usize; DMA_COUNTER_OPS_EXT];
    rows[DMA_COUNTER_MEMCPY] = memcpy;
    rows[DMA_COUNTER_MEMSET] = memset;
    rows[DMA_COUNTER_MEMCMP] = memcmp;
    rows[DMA_COUNTER_INPUTCPY] = inputcpy;
    rows[DMA_COUNTER_MEMCPY_8] = memcpy.div_ceil(8);
    rows[DMA_COUNTER_MEMSET_8] = memset.div_ceil(8);
    rows
}

fn plan(rows: &[usize]) -> Dma64AlignedInstances {
    let mut info = Dma64AlignedInstances::default();
    Strategy::calculate_dma_64_alignment_strategy(rows, &mut info);
    info
}

/// Every air a kind was routed to must be able to prove it, and the rows routed to each air must fit
/// in the instances the strategy asked for. This is exactly what `DmaInstancesBuilder` panics on when
/// the strategy over-promises.
fn assert_fits(rows: &[usize], info: &Dma64AlignedInstances) {
    let caps = caps();
    let mut routed = [0usize; air::COUNT];

    for (kind, &target) in info.assignment.iter().enumerate() {
        let (plain, packed) = match kind {
            kind::MEMCPY => (rows[DMA_COUNTER_MEMCPY], Some(rows[DMA_COUNTER_MEMCPY_8])),
            kind::MEMSET => (rows[DMA_COUNTER_MEMSET], Some(rows[DMA_COUNTER_MEMSET_8])),
            kind::MEMCMP => (rows[DMA_COUNTER_MEMCMP], None),
            kind::INPUTCPY => (rows[DMA_COUNTER_INPUTCPY], None),
            _ => unreachable!(),
        };
        if plain == 0 {
            continue;
        }

        match (kind, target) {
            (kind::MEMCPY, air::MEMCPY) => routed[target] += packed.unwrap(),
            (kind::MEMSET, air::MEMSET) => routed[target] += packed.unwrap(),
            (_, air::MEMCPY | air::MEMSET) => {
                std::panic!("kind {kind} routed to the packed air {target}, cannot prove it")
            }
            (kind::INPUTCPY, air::MEM | air::MEM_LARGE) => {
                std::panic!("an input copy was routed to a mem air, which cannot prove it")
            }
            _ => routed[target] += plain,
        }
    }

    for (a, &rows_in_air) in routed.iter().enumerate() {
        assert!(
            rows_in_air <= info.instances[a] * caps[a],
            "air {a} overflows: {rows_in_air} rows in {} instances of {}: {rows:?} → {info:?}",
            info.instances[a],
            caps[a],
        );
    }
}

fn total_instances(info: &Dma64AlignedInstances) -> usize {
    info.instances.iter().sum()
}

#[test]
fn no_rows_plans_nothing() {
    let rows = rows_of(0, 0, 0, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);
    assert_eq!(total_instances(&info), 0);
}

/// The criterion's headline: work that would need several short instances is given the tall air, even
/// though the memory is no smaller.
#[test]
fn a_big_memcmp_goes_to_the_tall_air() {
    let caps = caps();
    // More than the short general air holds, but within the tall one.
    let rows = rows_of(0, 0, caps[air::FULL] + 1, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(total_instances(&info), 1, "one tall instance beats two short ones");
    assert!(caps[air::FULL_LARGE] > caps[air::FULL]);
}

/// Once one instance is enough either way, the memory tie-break sends the work to the cheapest air
/// that can prove it — which is a question of measured instance cost, not of column count. The
/// general air is wider than the mem one but half as tall, and the height wins: `Dma64Aligned`
/// costs 7.58 GB against `Dma64AlignedMem`'s 11.78 GB (see `air_costs.rs`).
#[test]
fn area_breaks_the_tie_for_a_small_workload() {
    let rows = rows_of(0, 0, 1000, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(total_instances(&info), 1);

    let caps = caps();
    let airs = Strategy::dma_64_aligned_airs();
    let chosen = info.assignment[kind::MEMCMP];
    assert_eq!(chosen, air::FULL, "the short general air is the cheapest home for a memcmp");

    // Pin the reason rather than the outcome: no air that could also prove this memcmp in a single
    // instance is cheaper than the one picked.
    for candidate in [air::FULL_LARGE, air::FULL, air::MEM_LARGE, air::MEM] {
        if caps[candidate] >= 1000 {
            assert!(
                airs[candidate].memory >= airs[chosen].memory,
                "air {candidate} is cheaper than the chosen air {chosen}"
            );
        }
    }
}

/// Kinds that fit together share an instance rather than taking one each — which is the packing that
/// the instance-first criterion is there to find.
#[test]
fn kinds_share_an_instance_rather_than_opening_two() {
    let rows = rows_of(0, 0, 1000, 1000);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(total_instances(&info), 1, "memcmp rides with the input copy in the general air");
    assert_eq!(info.assignment[kind::MEMCMP], info.assignment[kind::INPUTCPY]);
}

/// An input copy has no specialised air, so it can only go to a general one however small it is.
#[test]
fn an_input_copy_only_ever_goes_to_a_general_air() {
    for inputcpy in [1, 1000, 10_000_000] {
        let rows = rows_of(0, 0, 0, inputcpy);
        let info = plan(&rows);
        assert_fits(&rows, &info);
        assert!(matches!(info.assignment[kind::INPUTCPY], air::FULL | air::FULL_LARGE));
    }
}

/// The packed airs hold eight operations per row, so a memcpy big enough to need several general
/// instances fits in one packed instance — fewer instances *and* less memory.
#[test]
fn a_big_memcpy_takes_the_packed_air() {
    let caps = caps();
    let rows = rows_of(4 * caps[air::MEMCPY], 0, 0, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(info.assignment[kind::MEMCPY], air::MEMCPY);
    assert_eq!(total_instances(&info), 1);
}

/// The rows each kind owes its air are what the per-chunk hand-out draws down, so they must be
/// counted in that air's own row cost — the packed count for a packed air, the plain one otherwise.
#[test]
fn the_owed_rows_are_counted_in_the_chosen_air_s_cost() {
    let caps = caps();
    let rows = rows_of(4 * caps[air::MEMCPY], 0, 0, 0);
    let info = plan(&rows);
    assert_eq!(info.assignment[kind::MEMCPY], air::MEMCPY);
    assert_eq!(info.rows[kind::MEMCPY], rows[DMA_COUNTER_MEMCPY_8]);

    let rows = rows_of(0, 0, 1000, 0);
    let info = plan(&rows);
    assert_eq!(info.rows[kind::MEMCMP], 1000);
}

/// Whatever the mix, the granted instances must hold everything routed to them — the invariant the
/// builders assert at hand-out time.
#[test]
fn capacity_invariant_holds_around_instance_boundaries() {
    let caps = caps();
    let values = [
        0,
        1,
        caps[air::MEM] - 1,
        caps[air::MEM],
        caps[air::MEM] + 1,
        caps[air::FULL_LARGE],
        caps[air::FULL_LARGE] + 1,
    ];
    for memcpy in values {
        for memset in values {
            for memcmp in values {
                for inputcpy in values {
                    let rows = rows_of(memcpy, memset, memcmp, inputcpy);
                    let info = plan(&rows);
                    assert_fits(&rows, &info);
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────── the fused air
//
// [`DmaStrategy::USE_DMA_WITH_PRE_POST`] decides whether the DMA controller and its PRE/POST
// sub-operations are proved by one air or by two, and `calculate` branches on it once for the
// sizing and once for the hand-out. What follows covers the branch the strategy ships with;
// `the_strategy_ships_with_the_fused_air` is what fails first if the constant is turned back off.

/// Rows one fused instance holds — the budget every hand-out below is measured against.
const WPP_ROWS: usize = Strategy::DMA_WITH_PRE_POST_ROWS;

/// The counters of one chunk holding `single` operations that need at most one of PRE/POST and
/// `double` that need both.
///
/// The `Dma` and `DmaPrePost` entries are filled in as the bus device fills them for the same
/// operations, so that "the two separate airs get nothing" means they were given something to
/// begin with.
fn wpp_chunk(single: usize, double: usize) -> Box<dyn BusDeviceMetrics> {
    let mut counter = DmaCounterInputGen::new(BusDeviceMode::Counter);
    counter.counters[DMA_WITH_PRE_POST_OFFSET + DMA_WPP_CLASS_SINGLE] = single;
    counter.counters[DMA_WITH_PRE_POST_OFFSET + DMA_WPP_CLASS_DOUBLE] = double;
    // One `Dma` row per operation; one `DmaPrePost` row per sub-operation, which is one for a
    // single and two for a double.
    counter.counters[DMA_OFFSET + DMA_COUNTER_MEMCPY] = single + double;
    counter.counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_MEMCPY] = single + 2 * double;
    Box::new(counter)
}

/// The plans `calculate` returns for the airs that are not the fused one, by air id.
type AirPlans = Vec<(usize, Vec<(CheckPoint, DmaCheckPoint)>)>;

/// Runs the whole strategy over `chunks`, each of them `(single, double)` operations, and hands
/// back the strategy — which holds the instance counts and the fused plan — with the plans of the
/// other airs.
fn run_strategy(chunks: &[(usize, usize)]) -> (Strategy, AirPlans) {
    let mut strategy = Strategy::default();
    let counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = chunks
        .iter()
        .enumerate()
        .map(|(chunk, &(single, double))| (ChunkId(chunk), wpp_chunk(single, double)))
        .collect();
    let plans = strategy.calculate(counters);
    (strategy, plans)
}

/// Rows each fused instance was handed, over every chunk and class.
fn wpp_rows_per_instance(plan: &[(CheckPoint, DmaWithPrePostCheckPoint)]) -> Vec<usize> {
    plan.iter()
        .map(|(_, cp)| {
            cp.chunks
                .values()
                .map(|(_, counters)| {
                    (0..DMA_WPP_CLASSES)
                        .map(|class| {
                            counters.classes[class].collect_count as usize
                                * DMA_WPP_CLASS_ROWS[class]
                        })
                        .sum::<usize>()
                })
                .sum()
        })
        .collect()
}

/// The switch is what the tests below describe, so it is also the first thing they check: turning
/// it off silently would leave them passing without ever reaching the fused air.
///
/// Asserting on a constant is the whole point here — the constant *is* what is under test — so the
/// lint that would otherwise flag it is off for this one assertion.
#[test]
#[allow(clippy::assertions_on_constants)]
fn the_strategy_ships_with_the_fused_air() {
    assert!(
        Strategy::USE_DMA_WITH_PRE_POST,
        "the fused air is off, so the tests below no longer cover the branch the strategy takes; \
         they describe the configuration where it is on and are what has to be revisited"
    );
}

/// The fused air proves the controller and its sub-operations together, so the two airs it
/// replaces have to come out of the plan empty — an instance of either would prove the same
/// operation a second time.
#[test]
fn the_fused_air_replaces_dma_and_dma_pre_post() {
    let (strategy, plans) = run_strategy(&[(1_000, 500)]);

    assert_eq!(strategy.dma, 0);
    assert_eq!(strategy.dma_pre_post, 0);
    assert_eq!(strategy.dma_with_pre_post, 1);

    let segments = |air_id: usize| plans.iter().find(|(id, _)| *id == air_id).unwrap().1.len();
    assert_eq!(segments(DmaTrace::<Goldilocks>::AIR_ID), 0, "the Dma air was given an instance");
    assert_eq!(
        segments(DmaPrePostTrace::<Goldilocks>::AIR_ID),
        0,
        "the DmaPrePost air was given an instance"
    );
    assert_eq!(strategy.dma_with_pre_post_plan.len(), 1);
}

/// Nothing to prove asks for nothing, in either air family.
#[test]
fn no_operation_plans_nothing() {
    let (strategy, _) = run_strategy(&[(0, 0), (0, 0)]);
    assert_eq!(strategy.dma_with_pre_post, 0);
    assert!(strategy.dma_with_pre_post_plan.is_empty());
}

/// The air is budgeted in rows, not in operations: a double costs two of them, so the same count
/// of operations needs more instances when they all carry both a PRE and a POST.
#[test]
fn the_budget_counts_rows_not_operations() {
    let ops = WPP_ROWS;
    let (singles, _) = run_strategy(&[(ops, 0)]);
    let (doubles, _) = run_strategy(&[(0, ops)]);

    assert_eq!(
        singles.dma_with_pre_post,
        DmaWithPrePostInstancesBuilder::instances_needed(ops, WPP_ROWS)
    );
    assert_eq!(
        doubles.dma_with_pre_post,
        DmaWithPrePostInstancesBuilder::instances_needed(2 * ops, WPP_ROWS)
    );
    assert!(doubles.dma_with_pre_post > singles.dma_with_pre_post);
}

/// Nothing may be lost and nothing proved twice: the instances that touch a chunk hand out exactly
/// the operations it counted, and each of them skips exactly what the ones before it took — so the
/// collected ranges tile the chunk with neither a gap nor an overlap.
#[test]
fn every_operation_is_planned_exactly_once() {
    let chunks = [(3, 5), (WPP_ROWS, 1), (0, WPP_ROWS), (7, 0)];
    let (strategy, _) = run_strategy(&chunks);
    let plan = &strategy.dma_with_pre_post_plan;

    // The tiling is only worth checking where there is something to tile: the chunks above are
    // sized to fill several instances and to leave at least one of them straddling a boundary.
    assert!(plan.len() > 1, "the plan collapsed to a single instance");
    assert!(
        (0..chunks.len()).any(|chunk| plan
            .iter()
            .filter(|(_, cp)| cp.chunks.contains_key(&ChunkId(chunk)))
            .count()
            > 1),
        "no chunk crosses an instance boundary"
    );

    for (chunk, &(single, double)) in chunks.iter().enumerate() {
        let chunk_id = ChunkId(chunk);
        for (class, expected) in [(DMA_WPP_CLASS_SINGLE, single), (DMA_WPP_CLASS_DOUBLE, double)] {
            // The instances holding a piece of this chunk, in segment order.
            let counters: Vec<_> = plan
                .iter()
                .filter_map(|(_, cp)| cp.chunks.get(&chunk_id))
                .map(|(_, counters)| counters.classes[class])
                .collect();

            let collected: usize = counters.iter().map(|c| c.collect_count as usize).sum();
            assert_eq!(collected, expected, "chunk {chunk}, class {class}");

            let mut taken = 0;
            for counter in counters {
                assert_eq!(
                    counter.initial_skip as usize, taken,
                    "chunk {chunk}, class {class}: the skip does not continue the previous instance"
                );
                taken += counter.collect_count as usize;
            }
        }
    }
}

/// `instances_needed` is the only budget the hand-out is given, so it has to hold whatever the mix
/// and wherever the chunks fall: no instance is ever handed more rows than it has.
#[test]
fn no_instance_is_given_more_rows_than_it_holds() {
    let half = WPP_ROWS / 2;
    for chunks in [
        vec![(WPP_ROWS, 0)],
        vec![(0, WPP_ROWS)],
        vec![(WPP_ROWS - 1, 1)],
        vec![(1, WPP_ROWS - 1)],
        // Odd boundaries: the singles leave each instance on an odd row, where a double no longer
        // fits and the last row is dropped.
        vec![(1, half), (1, half), (1, half)],
        vec![(half, half), (half, half), (1, 1)],
        // Empty chunks between full ones must not disturb the hand-out.
        vec![(0, 0), (WPP_ROWS + 1, 0), (0, 0)],
    ] {
        let (strategy, _) = run_strategy(&chunks);
        let rows = wpp_rows_per_instance(&strategy.dma_with_pre_post_plan);
        for (segment, used) in rows.iter().enumerate() {
            assert!(
                *used <= WPP_ROWS,
                "{chunks:?}: instance {segment} was given {used} rows out of {WPP_ROWS}"
            );
        }

        // And the hand-out stays between the two bounds it is squeezed by: it cannot do better
        // than a perfect packing, and it may not ask for more instances than it was budgeted.
        let total: usize = chunks.iter().map(|&(single, double)| single + 2 * double).sum();
        assert_eq!(rows.iter().sum::<usize>(), total, "{chunks:?}: rows were lost or duplicated");
        assert!(
            rows.len() >= total.div_ceil(WPP_ROWS) && rows.len() <= strategy.dma_with_pre_post,
            "{chunks:?}: {} instances for {total} rows, budgeted {}",
            rows.len(),
            strategy.dma_with_pre_post
        );
    }
}

/// The planner splits the operations by row cost and the collector picks them up by the same
/// split, so both have to read it out of the encoding the same way: what the counter files under
/// `DMA_WPP_CLASS_DOUBLE` is exactly what `DmaWithPrePostInput::is_double` keeps two rows for.
#[test]
fn the_counter_and_the_collector_agree_on_the_row_cost() {
    // dst offset, src offset and count, covering the four PRE/POST combinations. None of the four
    // is a direct operation, so all of them reach the fused air.
    let cases: [(u64, u64, usize, usize, usize); 4] = [
        // PRE and POST: 7 bytes to close the first word, two whole words, 3 left over.
        (1, 1, 7 + 16 + 3, 7, 3),
        // PRE only: the count ends on a word boundary.
        (1, 1, 7 + 16, 7, 0),
        // POST only: the dst starts on one.
        (0, 0, 16 + 3, 0, 3),
        // Neither, and still not direct: the src is not aligned with the dst.
        (0, 1, 16, 0, 0),
    ];

    for (dst_offset, src_offset, count, pre, post) in cases {
        let encoded = DmaInfo::encode_memcpy(0x8000 + dst_offset, 0x9000 + src_offset, count);
        let case = format!("dst+{dst_offset} src+{src_offset} count {count}");

        // Pin the case itself, so a wrong expectation cannot make the agreement below vacuous.
        assert_eq!(DmaInfo::get_pre_count(encoded), pre, "{case}: pre count");
        assert_eq!(DmaInfo::get_post_count(encoded), post, "{case}: post count");
        assert!(!DmaInfo::is_direct(encoded), "{case}: direct operations never reach the air");

        let is_double = pre > 0 && post > 0;
        assert_eq!(DmaWithPrePostInput::is_double(encoded), is_double, "{case}: collector");

        let mut counter = DmaCounterInputGen::new(BusDeviceMode::Counter);
        counter.inst_count(encoded, ZiskOp::DMA_MEMCPY, 0);
        let counted = (
            counter.counters[DMA_WITH_PRE_POST_OFFSET + DMA_WPP_CLASS_SINGLE],
            counter.counters[DMA_WITH_PRE_POST_OFFSET + DMA_WPP_CLASS_DOUBLE],
        );
        assert_eq!(counted, if is_double { (0, 1) } else { (1, 0) }, "{case}: counter");
    }
}
