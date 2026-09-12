//! What these cover is the one thing the fused air cannot do: split an operation. A two-row
//! operation puts its PRE row right after its DMA row, so when a single row is left the operation
//! has to start the next instance and that row is lost.

use super::*;
use zisk_common::ChunkId;

/// Rows every class of every chunk of a plan actually uses, per instance.
fn rows_per_instance(plan: &[(CheckPoint, DmaWithPrePostCheckPoint)]) -> Vec<usize> {
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

/// Operations of each class an instance collects, summed over its chunks.
fn ops_per_instance(plan: &[(CheckPoint, DmaWithPrePostCheckPoint)], class: usize) -> Vec<usize> {
    plan.iter()
        .map(|(_, cp)| {
            cp.chunks.values().map(|(_, c)| c.classes[class].collect_count as usize).sum()
        })
        .collect()
}

#[test]
fn single_row_operations_fill_every_row() {
    // 1 row each, so nothing is ever wasted.
    let rows = 8;
    let mut builder = DmaWithPrePostInstancesBuilder::new(
        "test",
        DmaWithPrePostInstancesBuilder::instances_needed(16, rows),
        rows,
    );
    builder.add_ops(ChunkId(0), DMA_WPP_CLASS_SINGLE, 16);
    let plan = builder.get_plan();
    assert_eq!(rows_per_instance(&plan), vec![8, 8]);
    assert_eq!(ops_per_instance(&plan, DMA_WPP_CLASS_SINGLE), vec![8, 8]);
}

#[test]
fn double_row_operations_are_never_split() {
    // 2 rows each and an even height: every instance is packed full.
    let rows = 8;
    let mut builder = DmaWithPrePostInstancesBuilder::new(
        "test",
        DmaWithPrePostInstancesBuilder::instances_needed(16, rows),
        rows,
    );
    builder.add_ops(ChunkId(0), DMA_WPP_CLASS_DOUBLE, 8);
    let plan = builder.get_plan();
    assert_eq!(rows_per_instance(&plan), vec![8, 8]);
    assert_eq!(ops_per_instance(&plan, DMA_WPP_CLASS_DOUBLE), vec![4, 4]);
}

#[test]
fn the_last_row_is_dropped_when_a_double_operation_does_not_fit() {
    // One single-row operation leaves the instance on an odd row, so the doubles that follow can
    // only take 6 of the remaining 7 rows: 1 + 3*2 = 7, and row 8 stays empty.
    let rows = 8;
    let mut builder = DmaWithPrePostInstancesBuilder::new("test", 4, rows);
    builder.add_ops(ChunkId(0), DMA_WPP_CLASS_SINGLE, 1);
    builder.add_ops(ChunkId(0), DMA_WPP_CLASS_DOUBLE, 5);
    let plan = builder.get_plan();

    // First instance: the single + 3 doubles = 7 rows, one row wasted.
    // Second instance: the remaining 2 doubles = 4 rows.
    assert_eq!(rows_per_instance(&plan), vec![7, 4]);
    assert_eq!(ops_per_instance(&plan, DMA_WPP_CLASS_SINGLE), vec![1, 0]);
    assert_eq!(ops_per_instance(&plan, DMA_WPP_CLASS_DOUBLE), vec![3, 2]);

    // No instance is ever asked for more rows than it has.
    for used in rows_per_instance(&plan) {
        assert!(used <= rows, "an instance was given {used} rows out of {rows}");
    }
}

#[test]
fn the_skip_of_the_second_instance_continues_the_first() {
    // The doubles of chunk 0 straddle the boundary: the second instance has to skip the ones the
    // first one already took, and only those — the skip is per class.
    let rows = 8;
    let mut builder = DmaWithPrePostInstancesBuilder::new("test", 4, rows);
    builder.add_ops(ChunkId(0), DMA_WPP_CLASS_SINGLE, 1);
    builder.add_ops(ChunkId(0), DMA_WPP_CLASS_DOUBLE, 5);
    let plan = builder.get_plan();

    let first = &plan[0].1.chunks[&ChunkId(0)].1;
    assert_eq!(first.classes[DMA_WPP_CLASS_SINGLE].initial_skip, 0);
    assert_eq!(first.classes[DMA_WPP_CLASS_DOUBLE].initial_skip, 0);

    let second = &plan[1].1.chunks[&ChunkId(0)].1;
    assert_eq!(second.classes[DMA_WPP_CLASS_SINGLE].initial_skip, 1);
    assert_eq!(second.classes[DMA_WPP_CLASS_DOUBLE].initial_skip, 3);
    assert_eq!(second.classes[DMA_WPP_CLASS_SINGLE].collect_count, 0);
    assert_eq!(second.classes[DMA_WPP_CLASS_DOUBLE].collect_count, 2);
}

#[test]
fn instances_needed_covers_the_wasted_rows() {
    // The worst case: every instance loses its last row to a double that does not fit. With a
    // height of 8 that is 7 useful rows per instance, which is what the bound has to allow for.
    const CHUNKS: usize = 4;
    let rows = 8;
    for singles in 0..4 {
        for doubles in 0..8 {
            let total_rows = (singles + doubles * 2) * CHUNKS;
            // `instances_needed` is the only budget the builder is given: if it were too tight,
            // `open_new_instance` would panic here.
            let mut builder = DmaWithPrePostInstancesBuilder::new(
                "test",
                DmaWithPrePostInstancesBuilder::instances_needed(total_rows, rows),
                rows,
            );
            // Interleaving the classes chunk by chunk is what forces the odd boundaries.
            for chunk in 0..CHUNKS {
                builder.add_ops(ChunkId(chunk), DMA_WPP_CLASS_SINGLE, singles);
                builder.add_ops(ChunkId(chunk), DMA_WPP_CLASS_DOUBLE, doubles);
            }
            let plan = builder.get_plan();
            for used in rows_per_instance(&plan) {
                assert!(
                    used <= rows,
                    "singles={singles} doubles={doubles}: an instance was given {used} rows                      out of {rows}"
                );
            }
        }
    }
}

#[test]
fn nothing_to_prove_plans_nothing() {
    let mut builder = DmaWithPrePostInstancesBuilder::new("test", 0, 8);
    builder.add_ops(ChunkId(0), DMA_WPP_CLASS_SINGLE, 0);
    builder.add_ops(ChunkId(0), DMA_WPP_CLASS_DOUBLE, 0);
    assert!(builder.get_plan().is_empty());
    assert_eq!(DmaWithPrePostInstancesBuilder::instances_needed(0, 8), 0);
}
