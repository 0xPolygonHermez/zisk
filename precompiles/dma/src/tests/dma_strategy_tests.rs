//! Unit tests for the `Dma`/`DmaPrePost` instance-count strategy.
//! Declared from `dma_strategy.rs` via `#[cfg(test)] #[path = …] mod tests;`, so it stays a child
//! module of `dma_strategy` and keeps `super::` access to privates.

use super::*;
use fields::Goldilocks;

type Strategy = DmaStrategy<Goldilocks>;

/// Rows per instance used by every air in these tests, so the three capacities are comparable.
const CAP: usize = 100;

/// Builds the `rows` slice `calculate_dma_strategy` expects, indexed by `DMA_COUNTER_*`.
fn rows_of(
    memcpy: usize,
    memset: usize,
    memcmp: usize,
    inputcpy: usize,
) -> [usize; DMA_COUNTER_OPS] {
    let mut rows = [0usize; DMA_COUNTER_OPS];
    rows[DMA_COUNTER_MEMCPY] = memcpy;
    rows[DMA_COUNTER_MEMSET] = memset;
    rows[DMA_COUNTER_MEMCMP] = memcmp;
    rows[DMA_COUNTER_INPUTCPY] = inputcpy;
    rows
}

fn plan(rows: &[usize]) -> DmaInstances {
    let mut info = DmaInstances::default();
    Strategy::calculate_dma_strategy(rows, CAP, CAP, CAP, &mut info);
    info
}

/// The rows routed to each air must fit in the instances the strategy asked for, and no row may be
/// routed twice. This is exactly what `DmaInstancesBuilder::open_new_instance` panics on when the
/// strategy over-promises free space.
fn assert_fits(rows: &[usize], info: &DmaInstances) {
    assert!(
        info.rows_memcpy_to_full <= rows[DMA_COUNTER_MEMCPY],
        "moved more memcpy rows to full than exist: {rows:?} → {info:?}"
    );
    assert!(
        info.rows_inputcpy_to_full <= rows[DMA_COUNTER_INPUTCPY],
        "moved more inputcpy rows to full than exist: {rows:?} → {info:?}"
    );

    let full_rows = rows[DMA_COUNTER_MEMSET]
        + rows[DMA_COUNTER_MEMCMP]
        + info.rows_memcpy_to_full
        + info.rows_inputcpy_to_full;
    assert!(
        full_rows <= info.full * CAP,
        "full air overflows: {full_rows} rows in {} instances: {rows:?} → {info:?}",
        info.full
    );

    let memcpy_rows = rows[DMA_COUNTER_MEMCPY] - info.rows_memcpy_to_full;
    assert!(
        memcpy_rows <= info.memcpy * CAP,
        "memcpy air overflows: {memcpy_rows} rows in {} instances: {rows:?} → {info:?}",
        info.memcpy
    );

    let inputcpy_rows = rows[DMA_COUNTER_INPUTCPY] - info.rows_inputcpy_to_full;
    assert!(
        inputcpy_rows <= info.inputcpy * CAP,
        "inputcpy air overflows: {inputcpy_rows} rows in {} instances: {rows:?} → {info:?}",
        info.inputcpy
    );
}

#[test]
fn exact_multiple_of_capacity_leaves_no_room_to_spare() {
    // memset+memcmp fill the last full instance exactly, so there is nowhere to put the memcpy
    // remainder: it must keep its own instance.
    let rows = rows_of(CAP + 50, CAP, 0, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(info.full, 1);
    assert_eq!(info.memcpy, 2);
    assert_eq!(info.rows_memcpy_to_full, 0);
}

#[test]
fn partially_filled_full_instance_absorbs_the_memcpy_remainder() {
    // 50 free rows in the single full instance, and a 30-row memcpy remainder: folding it in drops
    // one whole DmaMemCpy instance.
    let rows = rows_of(CAP + 30, 50, 0, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(info.full, 1);
    assert_eq!(info.memcpy, 1);
    assert_eq!(info.rows_memcpy_to_full, 30);
}

#[test]
fn both_remainders_share_one_extra_full_instance() {
    // No memset/memcmp at all, so there is no free space anywhere; pooling both remainders into a
    // new full instance still trades two partial instances for one.
    let rows = rows_of(CAP + 40, 0, 0, CAP + 50);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(info.full, 1);
    assert_eq!(info.memcpy, 1);
    assert_eq!(info.inputcpy, 1);
    assert_eq!(info.rows_memcpy_to_full, 40);
    assert_eq!(info.rows_inputcpy_to_full, 50);
}

#[test]
fn no_rows_plans_nothing() {
    let rows = rows_of(0, 0, 0, 0);
    let info = plan(&rows);
    assert_fits(&rows, &info);

    assert_eq!(info.full, 0);
    assert_eq!(info.memcpy, 0);
    assert_eq!(info.inputcpy, 0);
}

#[test]
fn capacity_invariant_holds_around_instance_boundaries() {
    // Sweep values just below, at and just above each multiple of the capacity — the exact-multiple
    // rows are where an off-by-one instance of free space hides.
    const VALUES: [usize; 9] = [0, 1, 99, 100, 101, 150, 199, 200, 201];
    for memcpy in VALUES {
        for memset in VALUES {
            for memcmp in VALUES {
                for inputcpy in VALUES {
                    let rows = rows_of(memcpy, memset, memcmp, inputcpy);
                    let info = plan(&rows);
                    assert_fits(&rows, &info);
                }
            }
        }
    }
}
