//! Unit tests for the DMA air-selection strategy.
//! Declared from `dma_strategy.rs` via `#[cfg(test)] #[path = …] mod tests;`, so it stays a child
//! module of `dma_strategy` and keeps `super::` access to privates.
//!
//! The instance costs of `DmaLoop` and `CompactDma` are estimates until a setup measures them (see
//! `air_costs.rs`), so the tests that pin where a workload goes are the ones the instance count
//! decides -- the first term of the criterion, which no cost can overturn. The ones that pin a
//! memory tie-break say which costs they compare.

use super::*;
use crate::{
    DmaLoopInput, DmaWithPrePostInput, DMA_LOOP_CLASSES, DMA_WPP_CLASS_DOUBLE, DMA_WPP_CLASS_SINGLE,
};
use proofman_fields::Goldilocks;
use zisk_core::zisk_ops::ZiskOp;
use zisk_precomp_helpers::DmaInfo;

type Strategy = DmaStrategy<Goldilocks>;

/// Only the 64-bit-aligned airs and `DmaUnaligned` for the loop, `DmaWithPrePost` for the
/// controller: the selection as it was before the loop airs.
const ALIGNED_ONLY: DmaAirOptions =
    DmaAirOptions { with_pre_post: true, dma_loop: false, compact_dma: false };
/// `DmaLoop` offered as well.
const WITH_LOOP: DmaAirOptions =
    DmaAirOptions { with_pre_post: true, dma_loop: true, compact_dma: false };
/// Every air.
const ALL: DmaAirOptions = DmaAirOptions { with_pre_post: true, dma_loop: true, compact_dma: true };

/// Rows one instance of each air holds, in [`air`] order, as the selection budgets them.
fn caps() -> [usize; air::COUNT] {
    Strategy::air_choices().map(|choice| choice.rows as usize)
}

/// The rows of kind `k` in `work`: what tells whether the kind has anything to place.
fn rows_of_kind(work: &DmaWork, k: usize) -> usize {
    if k == kind::CTRL {
        work.ctrl_rows
    } else {
        work.loop_rows[k]
    }
}

/// Aligned loop work. The packed counts are the plain ones: the packed airs hold four words per
/// row too (see `DmaCounterInputGen::OPS_X_ROW`).
fn aligned(memcpy: usize, memset: usize, memcmp: usize, inputcpy: usize) -> DmaWork {
    let mut work = DmaWork::default();
    work.loop_rows[kind::MEMCPY] = memcpy;
    work.loop_rows[kind::MEMSET] = memset;
    work.loop_rows[kind::MEMCMP] = memcmp;
    work.loop_rows[kind::INPUTCPY] = inputcpy;
    work.memcpy_packed = memcpy;
    work.memset_packed = memset;
    work
}

/// Every kind is routed to an air able to prove it, the rows routed to each air fit in the
/// instances opened for it, and the two blocks of `CompactDma` open the same instances. This is
/// exactly what the builders panic on when the strategy over-promises.
fn assert_fits(work: &DmaWork, options: DmaAirOptions, selection: &DmaSelection) {
    let caps = caps();
    let mut routed = [0usize; air::COUNT];
    for k in 0..kind::COUNT {
        if rows_of_kind(work, k) == 0 {
            continue;
        }
        let target = selection.assignment[k];
        let Some(&(_, rows)) =
            Strategy::options_of(work, k, options).iter().find(|(a, _)| *a == target)
        else {
            std::panic!("kind {k} routed to air {target}, which cannot prove it: {work:?}");
        };
        assert_eq!(selection.rows[k], rows as usize, "kind {k}: the rows owed to its air");
        routed[target] += rows as usize;
    }
    for (a, &rows) in routed.iter().enumerate() {
        assert!(
            rows <= selection.instances[a] * caps[a],
            "air {a} overflows: {rows} rows in {} instances of {}: {work:?} -> {selection}",
            selection.instances[a],
            caps[a],
        );
    }
    assert_eq!(
        selection.instances[air::COMPACT_WPP],
        selection.instances[air::COMPACT_LOOP],
        "the two blocks of CompactDma are one instance"
    );
}

/// Instances the selection opens, a fused air counted once.
fn total_instances(selection: &DmaSelection) -> usize {
    selection.instances.iter().sum::<usize>() - selection.instances[air::COMPACT_LOOP]
}

fn select(work: &DmaWork, options: DmaAirOptions) -> DmaSelection {
    let selection = Strategy::select(work, options);
    assert_fits(work, options, &selection);
    selection
}

/// The switches are what the tests below describe, so they are also the first thing checked:
/// turning one off silently would leave the tests of its branch passing without ever reaching it.
///
/// Asserting on a constant is the whole point here -- the constant *is* what is under test -- so
/// the lint that would otherwise flag it is off for this one test.
#[test]
#[allow(clippy::assertions_on_constants)]
fn the_strategy_ships_with_every_air() {
    assert_eq!(Strategy::OPTIONS, ALL, "the production options are not the ones tested as ALL");
}

// ───────────────────────────────────────────────────────── the 64-bit-aligned airs

#[test]
fn no_work_plans_nothing() {
    for options in [ALIGNED_ONLY, WITH_LOOP, ALL] {
        let selection = select(&DmaWork::default(), options);
        assert_eq!(total_instances(&selection), 0);
    }
}

/// The criterion's headline: work that would need several short instances is given a tall air,
/// even though the memory is no smaller.
#[test]
fn a_big_memcmp_goes_to_a_tall_air() {
    let caps = caps();
    // More than the short general air holds, but within the tall ones.
    let work = aligned(0, 0, caps[air::FULL] + 1, 0);
    let selection = select(&work, ALIGNED_ONLY);
    assert_eq!(total_instances(&selection), 1, "one tall instance beats two short ones");
    assert!(caps[selection.assignment[kind::MEMCMP]] > caps[air::FULL]);
}

/// Once one instance is enough either way, the memory tie-break sends the work to the cheapest air
/// that can prove it -- a question of measured instance cost, not of column count.
#[test]
fn memory_breaks_the_tie_for_a_small_workload() {
    let work = aligned(0, 0, 1000, 0);
    for options in [ALIGNED_ONLY, WITH_LOOP, ALL] {
        let selection = select(&work, options);
        assert_eq!(total_instances(&selection), 1);

        // Pin the reason rather than the outcome: no air that could also prove this memcmp in a
        // single instance is cheaper than the one picked.
        let airs = Strategy::air_choices();
        let chosen = selection.assignment[kind::MEMCMP];
        for (candidate, rows) in Strategy::options_of(&work, kind::MEMCMP, options) {
            if rows <= airs[candidate].rows {
                assert!(
                    airs[candidate].memory >= airs[chosen].memory,
                    "{options:?}: air {candidate} is cheaper than the chosen air {chosen}"
                );
            }
        }
    }
    // Among the aligned airs alone, that is the short general one.
    assert_eq!(select(&work, ALIGNED_ONLY).assignment[kind::MEMCMP], air::FULL);
}

/// Kinds that fit together share an instance rather than taking one each -- which is the packing
/// the instance-first criterion is there to find.
#[test]
fn kinds_share_an_instance_rather_than_opening_two() {
    let work = aligned(0, 0, 1000, 1000);
    for options in [ALIGNED_ONLY, WITH_LOOP, ALL] {
        let selection = select(&work, options);
        assert_eq!(total_instances(&selection), 1, "{options:?}: memcmp rides with the input copy");
        assert_eq!(selection.assignment[kind::MEMCMP], selection.assignment[kind::INPUTCPY]);
    }
}

/// An input copy has no specialised aligned air, so among those it only ever goes to a general one.
#[test]
fn an_input_copy_only_ever_goes_to_a_general_air() {
    for inputcpy in [1, 1000, 10_000_000] {
        let selection = select(&aligned(0, 0, 0, inputcpy), ALIGNED_ONLY);
        assert!(matches!(selection.assignment[kind::INPUTCPY], air::FULL | air::FULL_LARGE));
    }
}

/// A memcpy as tall as the packed air takes one instance of it: the cheapest of the airs that
/// hold it in one.
#[test]
fn a_big_memcpy_takes_the_packed_air() {
    let caps = caps();
    let work = aligned(caps[air::MEMCPY], 0, 0, 0);
    let selection = select(&work, ALL);
    assert_eq!(selection.assignment[kind::MEMCPY], air::MEMCPY);
    assert_eq!(total_instances(&selection), 1);
}

/// The rows each kind owes its air are what the per-chunk hand-out draws down, so they must be
/// counted in that air's own row cost -- the packed count for a packed air, the plain one
/// otherwise.
#[test]
fn the_owed_rows_are_counted_in_the_chosen_air_s_cost() {
    let caps = caps();
    // A packing twice as dense as the plain rows, so the two counts cannot be mistaken.
    let mut work = aligned(2 * caps[air::MEMCPY], 0, 0, 0);
    work.memcpy_packed = caps[air::MEMCPY];
    let selection = select(&work, ALL);
    assert_eq!(selection.assignment[kind::MEMCPY], air::MEMCPY);
    assert_eq!(selection.rows[kind::MEMCPY], work.memcpy_packed);

    let selection = select(&aligned(0, 0, 1000, 0), ALL);
    assert_eq!(selection.rows[kind::MEMCMP], 1000);
}

/// Whatever the mix, the granted instances must hold everything routed to them -- the invariant
/// the builders assert at hand-out time.
#[test]
fn capacity_invariant_holds_around_instance_boundaries() {
    let caps = caps();
    let values = [0, 1, caps[air::FULL] + 1, caps[air::FULL_LARGE] + 1];
    for options in [ALIGNED_ONLY, WITH_LOOP, ALL] {
        for memcpy in values {
            for memset in values {
                for memcmp in values {
                    for inputcpy in values {
                        let mut work = aligned(memcpy, memset, memcmp, inputcpy);
                        work.loop_rows[kind::MEMCPY_UNALIGNED] = caps[air::UNALIGNED] + 1;
                        work.loop_rows[kind::MEMCMP_UNALIGNED] = 7;
                        work.ctrl_rows = caps[air::COMPACT_WPP] + 1;
                        select(&work, options);
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────── the loop airs

/// `DmaLoop` proves the aligned and the unaligned loops alike, so one instance of it replaces an
/// aligned instance and an unaligned one.
#[test]
fn one_loop_instance_beats_an_aligned_and_an_unaligned_one() {
    let mut work = aligned(1000, 0, 0, 0);
    work.loop_rows[kind::MEMCPY_UNALIGNED] = 1000;

    let before = select(&work, ALIGNED_ONLY);
    assert_eq!(total_instances(&before), 2);

    let selection = select(&work, WITH_LOOP);
    assert_eq!(total_instances(&selection), 1);
    assert_eq!(selection.assignment[kind::MEMCPY], air::LOOP);
    assert_eq!(selection.assignment[kind::MEMCPY_UNALIGNED], air::LOOP);
    assert_eq!(selection.instances[air::LOOP], 1);
}

/// On a light workload the fused air is one instance where the others need a controller instance
/// and a loop instance: the controller and every loop kind go to its two blocks.
#[test]
fn a_light_mixed_workload_takes_one_compact_instance() {
    let mut work = aligned(50, 20, 0, 10);
    work.loop_rows[kind::MEMCMP_UNALIGNED] = 30;
    work.ctrl_rows = 100;

    let selection = select(&work, ALL);
    assert_eq!(total_instances(&selection), 1, "{selection}");
    assert_eq!(selection.assignment[kind::CTRL], air::COMPACT_WPP);
    for k in [kind::MEMCPY, kind::MEMSET, kind::INPUTCPY, kind::MEMCMP_UNALIGNED] {
        assert_eq!(selection.assignment[k], air::COMPACT_LOOP, "kind {k}");
    }
    assert_eq!(selection.instances[air::COMPACT_WPP], 1);
    assert_eq!(selection.instances[air::COMPACT_LOOP], 1);
}

/// The fused air is half as tall as `DmaWithPrePost`: a controller that fills one of those would
/// need two of it, so it stays where it is.
#[test]
fn a_heavy_controller_stays_in_dma_with_pre_post() {
    let caps = caps();
    let work = DmaWork { ctrl_rows: caps[air::WPP], ..Default::default() };
    assert!(work.ctrl_rows > caps[air::COMPACT_WPP]);

    let selection = select(&work, ALL);
    assert_eq!(selection.assignment[kind::CTRL], air::WPP);
    assert_eq!(total_instances(&selection), 1);
}

/// The planner sizes the loop airs from the counters and the collector cuts the inputs by
/// [`DmaLoopInput::total_rows`], so the two have to agree on every operation: what the counter
/// files under a kind is exactly the rows the collector takes for it.
#[test]
fn the_counter_and_the_loop_collector_agree_on_the_rows() {
    let mut ops = Vec::new();
    for dst_offset in 0..8u64 {
        for src_offset in 0..8u64 {
            for count in [8, 9, 17, 31, 40, 100] {
                let (dst, src) = (0x8000 + dst_offset, 0x9000 + src_offset);
                ops.push((ZiskOp::DMA_XMEMCPY, DmaInfo::encode_memcpy(dst, src, count)));
                ops.push((ZiskOp::DMA_XMEMCMP, DmaInfo::encode_memcmp(dst, src, count, 0)));
            }
        }
        for count in [8, 9, 17, 31, 40, 100] {
            let dst = 0x8000 + dst_offset;
            ops.push((ZiskOp::DMA_XMEMSET, DmaInfo::encode_memset(dst, count, 0xA5)));
            ops.push((ZiskOp::DMA_INPUTCPY, DmaInfo::encode_inputcpy(dst, count)));
        }
    }

    let mut classes_seen = [false; DMA_LOOP_CLASSES];
    for (op, encoded) in ops {
        let mut counter = DmaCounterInputGen::new(BusDeviceMode::Counter);
        counter.inst_count(encoded, op, 0);
        let work = DmaWork::from_counters(&counter.counters);

        let mut expected = [0; kind::LOOP];
        if let Some(class) = DmaLoopInput::class_of(op, encoded) {
            expected[class] = DmaLoopInput::total_rows(class, encoded);
            classes_seen[class] = true;
        }
        assert_eq!(work.loop_rows, expected, "0x{op:02X} {}", DmaInfo::to_string(encoded));
    }
    assert!(classes_seen.iter().all(|&seen| seen), "a class was never exercised");
}

// ─────────────────────────────────────────────────────────────────────────── the controller
//
// [`DmaStrategy::USE_DMA_WITH_PRE_POST`] decides whether the DMA controller and its PRE/POST
// sub-operations are proved by one air or by two. With it on, the controller goes to
// `DmaWithPrePost` or to the `wpp_` block of `CompactDma`, whichever the selection picks.

/// Rows one `DmaWithPrePost` instance holds -- the budget every hand-out below is measured against.
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

/// The plans `calculate` returns for the airs without a checkpoint type of their own, by air id.
type AirPlans = Vec<(usize, Vec<(CheckPoint, DmaCheckPoint)>)>;

/// Runs the whole strategy over `chunks`, each of them `(single, double)` operations.
fn run_strategy(chunks: &[(usize, usize)], options: DmaAirOptions) -> (Strategy, AirPlans) {
    let mut strategy = Strategy::default();
    let counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = chunks
        .iter()
        .enumerate()
        .map(|(chunk, &(single, double))| (ChunkId(chunk), wpp_chunk(single, double)))
        .collect();
    let plans = strategy.calculate_with(counters, options);
    (strategy, plans)
}

/// The plans of the air the controller went to, its height, and the instances it was budgeted.
fn ctrl_plans(strategy: &Strategy) -> (Vec<&DmaWithPrePostCheckPoint>, usize, usize) {
    if strategy.selection.assignment[kind::CTRL] == air::COMPACT_WPP {
        (
            strategy.compact_dma_plan.iter().map(|(_, cp)| &cp.wpp).collect(),
            Strategy::COMPACT_DMA_ROWS,
            strategy.compact_dma,
        )
    } else {
        (
            strategy.dma_with_pre_post_plan.iter().map(|(_, cp)| cp).collect(),
            WPP_ROWS,
            strategy.dma_with_pre_post,
        )
    }
}

/// Rows each instance was handed, over every chunk and class.
fn wpp_rows_per_instance(plan: &[&DmaWithPrePostCheckPoint]) -> Vec<usize> {
    plan.iter()
        .map(|cp| {
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

/// The fused controller air proves the controller and its sub-operations together, so the two
/// airs it replaces have to come out of the plan empty -- an instance of either would prove the
/// same operation a second time.
#[test]
fn the_fused_controller_replaces_dma_and_dma_pre_post() {
    for options in [ALIGNED_ONLY, ALL] {
        let (strategy, plans) = run_strategy(&[(1_000, 500)], options);

        assert_eq!(strategy.dma, 0);
        assert_eq!(strategy.dma_pre_post, 0);
        let (plan, _, instances) = ctrl_plans(&strategy);
        assert_eq!(instances, 1);
        assert_eq!(plan.len(), 1);

        let segments = |air_id: usize| plans.iter().find(|(id, _)| *id == air_id).unwrap().1.len();
        assert_eq!(segments(DmaTrace::<Goldilocks>::AIR_ID), 0, "the Dma air got an instance");
        assert_eq!(
            segments(DmaPrePostTrace::<Goldilocks>::AIR_ID),
            0,
            "the DmaPrePost air got an instance"
        );
    }
}

/// With the fused controller off, the controller goes back to `Dma` + `DmaPrePost`, which take
/// every operation, and neither air with a `wpp_` block gets anything.
#[test]
fn without_the_fused_controller_dma_and_dma_pre_post_take_it() {
    let options = DmaAirOptions { with_pre_post: false, dma_loop: true, compact_dma: false };
    let (strategy, plans) = run_strategy(&[(1_000, 500)], options);
    assert_eq!(strategy.dma, 1);
    assert_eq!(strategy.dma_pre_post, 1);
    assert_eq!(strategy.dma_with_pre_post, 0);
    assert_eq!(strategy.compact_dma, 0);
    assert!(strategy.dma_with_pre_post_plan.is_empty() && strategy.compact_dma_plan.is_empty());
    let segments = |air_id: usize| plans.iter().find(|(id, _)| *id == air_id).unwrap().1.len();
    assert_eq!(segments(DmaTrace::<Goldilocks>::AIR_ID), 1);
    assert_eq!(segments(DmaPrePostTrace::<Goldilocks>::AIR_ID), 1);
}

/// Nothing to prove asks for nothing, in any air family.
#[test]
fn no_operation_plans_nothing() {
    for options in [ALIGNED_ONLY, ALL] {
        let (strategy, _) = run_strategy(&[(0, 0), (0, 0)], options);
        assert_eq!(strategy.dma_with_pre_post + strategy.compact_dma + strategy.dma_loop, 0);
        assert!(strategy.dma_with_pre_post_plan.is_empty());
        assert!(strategy.compact_dma_plan.is_empty());
        assert!(strategy.dma_loop_plan.is_empty());
    }
}

/// The air is budgeted in rows, not in operations: a double costs two of them, so the same count
/// of operations needs more instances when they all carry both a PRE and a POST.
#[test]
fn the_budget_counts_rows_not_operations() {
    let ops = WPP_ROWS;
    let (singles, _) = run_strategy(&[(ops, 0)], ALIGNED_ONLY);
    let (doubles, _) = run_strategy(&[(0, ops)], ALIGNED_ONLY);

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
/// the operations it counted, and each of them skips exactly what the ones before it took -- so the
/// collected ranges tile the chunk with neither a gap nor an overlap.
///
/// Only `DmaWithPrePost` ever gets a controller that needs several instances: the fused air is half
/// as tall, so what fills two of its instances fits in one of `DmaWithPrePost`. Its `wpp_` block is
/// handed out by the same builder at its own height.
#[test]
fn every_operation_is_planned_exactly_once() {
    for (options, chunks) in [
        (ALIGNED_ONLY, vec![(3, 5), (WPP_ROWS, 1), (0, WPP_ROWS), (7, 0)]),
        (ALL, vec![(3, 5), (WPP_ROWS, 1), (0, WPP_ROWS), (7, 0)]),
    ] {
        let (strategy, _) = run_strategy(&chunks, options);
        let (plan, _, _) = ctrl_plans(&strategy);

        // The tiling is only worth checking where there is something to tile: the chunks above
        // are sized to fill several instances and to leave one of them straddling a boundary.
        assert!(plan.len() > 1, "{options:?}: the plan collapsed to a single instance");
        assert!(
            (0..chunks.len()).any(|chunk| plan
                .iter()
                .filter(|cp| cp.chunks.contains_key(&ChunkId(chunk)))
                .count()
                > 1),
            "{options:?}: no chunk crosses an instance boundary"
        );

        for (chunk, &(single, double)) in chunks.iter().enumerate() {
            let chunk_id = ChunkId(chunk);
            for (class, expected) in
                [(DMA_WPP_CLASS_SINGLE, single), (DMA_WPP_CLASS_DOUBLE, double)]
            {
                // The instances holding a piece of this chunk, in segment order.
                let counters: Vec<_> = plan
                    .iter()
                    .filter_map(|cp| cp.chunks.get(&chunk_id))
                    .map(|(_, counters)| counters.classes[class])
                    .collect();

                let collected: usize = counters.iter().map(|c| c.collect_count as usize).sum();
                assert_eq!(collected, expected, "{options:?}: chunk {chunk}, class {class}");

                let mut taken = 0;
                for counter in counters {
                    assert_eq!(
                        counter.initial_skip as usize, taken,
                        "{options:?}: chunk {chunk}, class {class}: the skip does not continue \
                         the previous instance"
                    );
                    taken += counter.collect_count as usize;
                }
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
        // Small ones, which the fused air takes when it is offered.
        vec![(1000, 1000), (0, 3), (5, 0)],
    ] {
        for options in [ALIGNED_ONLY, ALL] {
            let (strategy, _) = run_strategy(&chunks, options);
            let (plan, height, budgeted) = ctrl_plans(&strategy);
            let rows = wpp_rows_per_instance(&plan);
            for (segment, used) in rows.iter().enumerate() {
                assert!(
                    *used <= height,
                    "{chunks:?} {options:?}: instance {segment} was given {used} rows out of \
                     {height}"
                );
            }

            // And the hand-out stays between the two bounds it is squeezed by: it cannot do
            // better than a perfect packing, and it may not ask for more instances than it was
            // budgeted.
            let total: usize = chunks.iter().map(|&(single, double)| single + 2 * double).sum();
            assert_eq!(rows.iter().sum::<usize>(), total, "{chunks:?}: rows lost or duplicated");
            assert!(
                rows.len() >= total.div_ceil(height) && rows.len() <= budgeted,
                "{chunks:?} {options:?}: {} instances for {total} rows, budgeted {budgeted}",
                rows.len(),
            );
        }
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

// ─────────────────────────────────────────────────────────────── the loop hand-out

/// The counters of one chunk holding `ops`, as the bus device counts them.
fn counted(ops: &[(u8, u64)]) -> Box<dyn BusDeviceMetrics> {
    let mut counter = DmaCounterInputGen::new(BusDeviceMode::Counter);
    for &(op, encoded) in ops {
        counter.inst_count(encoded, op, 0);
    }
    Box::new(counter)
}

/// A loop of `rows` rows: an unaligned memcpy (`dst` aligned, `src` one byte off) of `4 * rows - 1`
/// words, whose extra read closes the last row.
fn unaligned_memcpy(rows: usize) -> (u8, u64) {
    let encoded = DmaInfo::encode_memcpy(0x8000, 0x9001, (4 * rows - 1) * 8);
    assert_eq!(DmaLoopInput::total_rows(kind::MEMCPY_UNALIGNED, encoded), rows);
    (ZiskOp::DMA_XMEMCPY, encoded)
}

/// An aligned memset of `rows` rows.
fn memset(rows: usize) -> (u8, u64) {
    let encoded = DmaInfo::encode_memset(0x8000, 4 * rows * 8, 0x5A);
    assert_eq!(DmaLoopInput::total_rows(kind::MEMSET, encoded), rows);
    (ZiskOp::DMA_XMEMSET, encoded)
}

/// Every loop row of every chunk goes to the air its kind was routed to, exactly once: per chunk
/// and kind, the instances of that air collect what the chunk counted, each skipping what the ones
/// before it took, no instance past its height -- and nothing is left over of what the selection
/// routed.
fn assert_loop_rows_planned_once(chunks: &[Vec<(u8, u64)>], options: DmaAirOptions) -> Strategy {
    let mut strategy = Strategy::default();
    let counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> =
        chunks.iter().enumerate().map(|(chunk, ops)| (ChunkId(chunk), counted(ops))).collect();
    strategy.calculate_with(counters, options);

    for k in 0..kind::LOOP {
        assert_eq!(strategy.selection.rows[k], 0, "kind {k}: rows routed but never handed out");
    }

    let loop_plans = |target: usize| -> (Vec<&DmaLoopCheckPoint>, usize) {
        match target {
            air::LOOP => {
                (strategy.dma_loop_plan.iter().map(|(_, cp)| cp).collect(), Strategy::DMA_LOOP_ROWS)
            }
            air::COMPACT_LOOP => (
                strategy.compact_dma_plan.iter().map(|(_, cp)| &cp.lp).collect(),
                Strategy::COMPACT_DMA_ROWS,
            ),
            _ => (Vec::new(), 0),
        }
    };

    for target in [air::LOOP, air::COMPACT_LOOP] {
        let (plan, height) = loop_plans(target);
        for (segment, cp) in plan.iter().enumerate() {
            let used: u64 = cp.chunks.values().map(|(_, c)| c.total_collect_count()).sum();
            assert!(used as usize <= height, "air {target}: instance {segment} got {used} rows");
        }
    }

    for (chunk, ops) in chunks.iter().enumerate() {
        let chunk_id = ChunkId(chunk);
        let mut expected = [0usize; DMA_LOOP_CLASSES];
        for &(op, encoded) in ops {
            if let Some(class) = DmaLoopInput::class_of(op, encoded) {
                expected[class] += DmaLoopInput::total_rows(class, encoded);
            }
        }
        for (class, &expected) in expected.iter().enumerate() {
            let target = strategy.selection.assignment[class];
            let (plan, _) = loop_plans(target);
            if plan.is_empty() {
                continue; // routed to an air of the other family
            }
            let counters: Vec<_> = plan
                .iter()
                .filter_map(|cp| cp.chunks.get(&chunk_id))
                .map(|(_, counters)| counters.classes[class])
                .collect();
            let mut taken = 0;
            for counter in &counters {
                assert_eq!(counter.initial_skip as usize, taken, "chunk {chunk}, class {class}");
                taken += counter.collect_count as usize;
            }
            assert_eq!(taken, expected, "chunk {chunk}, class {class}: rows lost or duplicated");
        }
    }
    strategy
}

/// More unaligned loop rows than a `DmaUnaligned` instance holds go to `DmaLoop`, whose instances
/// are twice as tall, and the hand-out cuts them across its instances without losing a row.
#[test]
fn the_loop_hand_out_tiles_the_chunks() {
    let rows = Strategy::DMA_LOOP_ROWS;
    let chunks = vec![
        vec![unaligned_memcpy(rows / 2 + 3), memset(5)],
        vec![unaligned_memcpy(rows - 7)],
        vec![],
        vec![unaligned_memcpy(rows / 3), unaligned_memcpy(11)],
    ];
    for options in [WITH_LOOP, ALL] {
        let strategy = assert_loop_rows_planned_once(&chunks, options);
        assert_eq!(strategy.selection.assignment[kind::MEMCPY_UNALIGNED], air::LOOP, "{options:?}");
        assert!(strategy.dma_loop_plan.len() > 1, "{options:?}: nothing was cut");
        assert!(strategy.dma_loop_plan.last().unwrap().1.is_last_segment);
        assert!(strategy.dma_loop_plan.iter().rev().skip(1).all(|(_, cp)| !cp.is_last_segment));
    }
}

/// A light mixed workload lands in one `CompactDma` instance, whose checkpoint carries the plan of
/// both blocks and lists every chunk either of them collects from.
#[test]
fn the_compact_plan_carries_both_blocks() {
    let chunk_counters = || -> Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> {
        vec![
            (ChunkId(0), counted(&[memset(3)])),
            (ChunkId(1), wpp_chunk(40, 2)),
            (ChunkId(2), counted(&[unaligned_memcpy(6), memset(2)])),
        ]
    };
    let mut strategy = Strategy::default();
    let plans = strategy.calculate_with(chunk_counters(), ALL);

    assert_eq!(strategy.compact_dma, 1, "{}", strategy.selection);
    assert_eq!(strategy.dma_with_pre_post + strategy.dma_loop + strategy.dma_unaligned, 0);
    assert!(plans.iter().all(|(_, plan)| plan.is_empty()), "another air got an instance");

    let (check_point, cp) = &strategy.compact_dma_plan[0];
    let chunks = |check_point: &CheckPoint| match check_point {
        CheckPoint::Multiple(chunks) => chunks.clone(),
        other => std::panic!("unexpected check point {other:?}"),
    };
    assert_eq!(chunks(check_point), vec![ChunkId(0), ChunkId(1), ChunkId(2)]);
    assert!(cp.wpp.is_last_segment && cp.lp.is_last_segment);

    // The controller block takes every operation of every chunk that is not direct, the
    // unaligned memcpy of chunk 2 included; the aligned memsets are direct and have none.
    for (chunk_id, counter) in &chunk_counters() {
        let counter = (**counter).as_any().downcast_ref::<DmaCounterInputGen>().unwrap();
        for class in [DMA_WPP_CLASS_SINGLE, DMA_WPP_CLASS_DOUBLE] {
            let ops = counter.counters[DMA_WITH_PRE_POST_OFFSET + class];
            let collected =
                cp.wpp.chunks.get(chunk_id).map_or(0, |(_, c)| c.classes[class].collect_count);
            assert_eq!(collected as usize, ops, "{chunk_id:?}, class {class}");
        }
    }
    assert!(!cp.wpp.chunks.contains_key(&ChunkId(0)), "a direct memset reached the controller");
    let (_, wpp) = cp.wpp.chunks[&ChunkId(1)];
    assert_eq!(wpp.classes[DMA_WPP_CLASS_SINGLE].collect_count, 40);
    assert_eq!(wpp.classes[DMA_WPP_CLASS_DOUBLE].collect_count, 2);

    let loop_rows = |chunk: usize, class: usize| {
        cp.lp.chunks.get(&ChunkId(chunk)).map_or(0, |(_, c)| c.classes[class].collect_count)
    };
    assert_eq!(loop_rows(0, kind::MEMSET), 3);
    assert_eq!(loop_rows(1, kind::MEMSET), 0);
    assert_eq!(loop_rows(2, kind::MEMSET), 2);
    assert_eq!(loop_rows(2, kind::MEMCPY_UNALIGNED), 6);
}

/// The blocks of `CompactDma` are planned by builders of their own and may run out apart: every
/// instance takes the next plan of each block, or an empty one once that block is done, and the
/// loop block is a segment of its chain in every instance -- the last one closes it.
#[test]
fn fusing_the_block_plans_pads_the_block_that_runs_out_first() {
    // Ten single-row operations in instances of 4 rows: three controller instances. One loop
    // instance.
    let mut wpp = DmaWithPrePostInstancesBuilder::new("wpp", 3, 4);
    wpp.add_ops(ChunkId(0), DMA_WPP_CLASS_SINGLE, 3);
    wpp.add_ops(ChunkId(1), DMA_WPP_CLASS_SINGLE, 7);
    let mut lp = DmaLoopInstancesBuilder::new("loop", 1, 4);
    lp.add_class_rows(ChunkId(2), kind::MEMSET, 4, 1);
    let (wpp, lp) = (wpp.get_plan(), lp.get_plan());
    assert_eq!((wpp.len(), lp.len()), (3, 1));

    let fused = fuse_compact_plans(wpp, lp);
    assert_eq!(fused.len(), 3);
    for (segment, (check_point, cp)) in fused.iter().enumerate() {
        let is_last = segment == 2;
        assert_eq!(cp.wpp.is_last_segment, is_last, "segment {segment}");
        assert_eq!(cp.lp.is_last_segment, is_last, "segment {segment}");
        let CheckPoint::Multiple(chunks) = check_point else { std::panic!("{check_point:?}") };
        let mut expected: Vec<ChunkId> =
            cp.wpp.chunks.keys().chain(cp.lp.chunks.keys()).copied().collect();
        expected.sort_unstable();
        expected.dedup();
        assert_eq!(chunks, &expected, "segment {segment}: the chunks of both blocks");
    }
    assert!(fused[0].1.lp.chunks.contains_key(&ChunkId(2)));
    assert!(fused[1].1.lp.chunks.is_empty() && fused[2].1.lp.chunks.is_empty());
}
