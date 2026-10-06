//! Unit tests for `DmaInstancesBuilder`'s per-(instance, chunk) bookkeeping.
//! Declared from `dma_instances_builder.rs` via `#[cfg(test)] #[path = …] mod tests;`.

use super::*;

/// Rows per instance used by every builder in these tests.
const ROWS: usize = 10;

#[test]
fn skip_applies_once_when_a_batch_crosses_an_instance_boundary() {
    let mut builder = DmaInstancesBuilder::new("test", 2, ROWS);
    // 15 memcpy rows for one chunk, whose first 5 rows were already collected by another air.
    builder.add_op_rows(ChunkId(0), 5, 15, 15, DMA_COUNTER_MEMCPY);
    let plan = builder.get_plan();
    assert_eq!(plan.len(), 2);

    let first = plan[0].1.chunks[&ChunkId(0)].1.memcpy;
    assert_eq!((first.initial_skip, first.collect_count), (5, 10));

    // The second instance skips the 5 rows taken elsewhere plus the 10 the first instance already
    // collected — the batch's skip is not charged a second time.
    let second = plan[1].1.chunks[&ChunkId(0)].1.memcpy;
    assert_eq!((second.initial_skip, second.collect_count), (15, 5));
}

#[test]
fn skip_is_kept_when_the_batch_fits_in_one_instance() {
    let mut builder = DmaInstancesBuilder::new("test", 1, ROWS);
    builder.add_op_rows(ChunkId(0), 3, 4, 4, DMA_COUNTER_MEMCPY);
    let plan = builder.get_plan();
    assert_eq!(plan.len(), 1);

    let counter = plan[0].1.chunks[&ChunkId(0)].1.memcpy;
    assert_eq!((counter.initial_skip, counter.collect_count), (3, 4));
}

#[test]
fn inputs_counter_is_scoped_to_its_chunk() {
    let mut builder = DmaInstancesBuilder::new("test", 1, ROWS);
    builder.add_op_rows(ChunkId(0), 0, 4, 2, DMA_COUNTER_MEMCPY);
    builder.add_op_rows(ChunkId(1), 0, 3, 1, DMA_COUNTER_MEMCPY);
    let plan = builder.get_plan();
    assert_eq!(plan.len(), 1);

    // Each record carries only its own inputs: chunk 1 must not inherit chunk 0's.
    let chunks = &plan[0].1.chunks;
    assert_eq!(chunks[&ChunkId(0)].0, 2);
    assert_eq!(chunks[&ChunkId(1)].0, 1);
}

#[test]
fn inputs_counter_restarts_on_a_new_instance() {
    let mut builder = DmaInstancesBuilder::new("test", 2, ROWS);
    // Chunk 0 fills the first instance exactly, so chunk 1 opens a second one.
    builder.add_op_rows(ChunkId(0), 0, ROWS, 3, DMA_COUNTER_MEMCPY);
    builder.add_op_rows(ChunkId(1), 0, 2, 1, DMA_COUNTER_MEMCPY);
    let plan = builder.get_plan();
    assert_eq!(plan.len(), 2);

    assert_eq!(plan[0].1.chunks[&ChunkId(0)].0, 3);
    assert_eq!(plan[1].1.chunks[&ChunkId(1)].0, 1);
}

#[test]
fn inputs_counter_sums_the_ops_of_the_same_chunk() {
    let mut builder = DmaInstancesBuilder::new("test", 1, ROWS);
    builder.add_op_rows(ChunkId(0), 0, 2, 2, DMA_COUNTER_MEMCPY);
    builder.add_op_rows(ChunkId(0), 0, 3, 1, DMA_COUNTER_MEMCMP);
    let plan = builder.get_plan();
    assert_eq!(plan.len(), 1);

    let (inputs, counters) = plan[0].1.chunks[&ChunkId(0)];
    assert_eq!(inputs, 3);
    assert_eq!(counters.memcpy.collect_count, 2);
    assert_eq!(counters.memcmp.collect_count, 3);
}
