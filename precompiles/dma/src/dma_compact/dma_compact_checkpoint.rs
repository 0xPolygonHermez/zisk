//! Plan of a `DmaCompact` instance: which operations of which chunks it proves.
//!
//! The other DMA machines plan in rows and let the collectors cut an operation wherever the row
//! budget of an instance ends (the AIRs have continuations for that). `DmaCompact` never splits an
//! operation, so its plan is expressed in OPERATIONS: for every chunk, skip the first `skip_ops`
//! DMA operations of the chunk and take the next `count_ops`. The counter records the rows of
//! every DMA operation of a chunk, in bus order, and [`DmaCompactInstancesBuilder`] packs them
//! into instances with that information.

use std::collections::HashMap;

use zisk_common::{CheckPoint, ChunkId};

/// What one instance collects from one chunk.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DmaCompactChunkPlan {
    /// DMA operations of the chunk (in bus order) that belong to previous instances.
    pub skip_ops: u32,
    /// DMA operations to collect after the skipped ones.
    pub count_ops: u32,
    /// Rows those operations occupy.
    pub rows: u32,
}

/// Plan of one `DmaCompact` instance.
#[derive(Debug, Default)]
pub struct DmaCompactCheckPoint {
    pub chunks: HashMap<ChunkId, DmaCompactChunkPlan>,
    /// Rows used by the instance.
    pub rows: usize,
}

impl DmaCompactCheckPoint {
    #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_plans"))]
    pub fn get_debug_info(&self, title: &str, segment_id: u64) -> String {
        let mut chunks: Vec<_> = self.chunks.iter().collect();
        chunks.sort_by_key(|(chunk_id, _)| **chunk_id);
        chunks
            .iter()
            .map(|(chunk_id, plan)| {
                format!(
                    "{title} #{segment_id}@{chunk_id} [ops:({}){}|rows:{}]",
                    plan.skip_ops, plan.count_ops, plan.rows
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Packs DMA operations into `DmaCompact` instances, in bus order and without splitting any.
///
/// An operation that does not fit in the rows left of the current instance closes it (the rows
/// left are wasted) and starts the next one. The caller guarantees that every operation fits in
/// an empty instance.
#[derive(Debug)]
pub struct DmaCompactInstancesBuilder {
    rows_x_instance: usize,
    rows_available: usize,
    instances: Vec<DmaCompactCheckPoint>,
    /// Chunk being added and what the current instance takes from it.
    current: Option<(ChunkId, DmaCompactChunkPlan)>,
}

impl DmaCompactInstancesBuilder {
    pub fn new(rows_x_instance: usize) -> Self {
        Self { rows_x_instance, rows_available: 0, instances: Vec::new(), current: None }
    }

    /// Number of instances packed so far.
    pub fn num_instances(&self) -> usize {
        self.instances.len()
    }

    /// Stores what the current instance takes from the current chunk, if anything.
    fn flush_chunk(&mut self) {
        if let Some((chunk_id, plan)) = &mut self.current {
            if plan.count_ops > 0 {
                let instance = self.instances.last_mut().expect("an open instance");
                instance.chunks.insert(*chunk_id, *plan);
                instance.rows += plan.rows as usize;
                // whatever comes next from this chunk goes to another instance
                plan.skip_ops += plan.count_ops;
                plan.count_ops = 0;
                plan.rows = 0;
            }
        }
    }

    /// Adds the next DMA operation of `chunk_id`, which takes `rows` rows. Operations must be
    /// added chunk by chunk, in bus order.
    pub fn add_op(&mut self, chunk_id: ChunkId, rows: usize) {
        assert!(
            rows <= self.rows_x_instance,
            "a DmaCompact operation of {rows} rows does not fit in an instance of {} rows",
            self.rows_x_instance
        );
        match &self.current {
            Some((current_chunk, _)) if *current_chunk == chunk_id => {}
            _ => {
                self.flush_chunk();
                self.current = Some((chunk_id, DmaCompactChunkPlan::default()));
            }
        }
        if rows > self.rows_available {
            self.flush_chunk();
            self.instances.push(DmaCompactCheckPoint::default());
            self.rows_available = self.rows_x_instance;
        }
        let (_, plan) = self.current.as_mut().unwrap();
        plan.count_ops += 1;
        plan.rows += rows as u32;
        self.rows_available -= rows;
    }

    /// The plan of every instance, in order.
    pub fn get_plan(mut self) -> Vec<(CheckPoint, DmaCompactCheckPoint)> {
        self.flush_chunk();
        self.instances
            .into_iter()
            .map(|instance| {
                let mut keys: Vec<_> = instance.chunks.keys().cloned().collect();
                keys.sort();
                (CheckPoint::Multiple(keys), instance)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_of(
        plan: &[(CheckPoint, DmaCompactCheckPoint)],
        instance: usize,
        chunk: usize,
    ) -> DmaCompactChunkPlan {
        plan[instance].1.chunks[&ChunkId(chunk)]
    }

    /// Operations are packed in order, an operation that does not fit opens a new instance, and
    /// the skip/count of a chunk split between instances are consistent.
    #[test]
    fn packs_without_splitting() {
        let mut builder = DmaCompactInstancesBuilder::new(10);
        // chunk 0: 4 + 4 fit in the first instance, 3 does not
        builder.add_op(ChunkId(0), 4);
        builder.add_op(ChunkId(0), 4);
        builder.add_op(ChunkId(0), 3);
        // chunk 1: 7 fits in the second one (3 + 7), 1 opens a third
        builder.add_op(ChunkId(1), 7);
        builder.add_op(ChunkId(1), 1);
        assert_eq!(builder.num_instances(), 3);

        let plan = builder.get_plan();
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[0].0, CheckPoint::Multiple(vec![ChunkId(0)]));
        assert_eq!(plan[1].0, CheckPoint::Multiple(vec![ChunkId(0), ChunkId(1)]));
        assert_eq!(plan[2].0, CheckPoint::Multiple(vec![ChunkId(1)]));

        assert_eq!(
            plan_of(&plan, 0, 0),
            DmaCompactChunkPlan { skip_ops: 0, count_ops: 2, rows: 8 }
        );
        assert_eq!(
            plan_of(&plan, 1, 0),
            DmaCompactChunkPlan { skip_ops: 2, count_ops: 1, rows: 3 }
        );
        assert_eq!(
            plan_of(&plan, 1, 1),
            DmaCompactChunkPlan { skip_ops: 0, count_ops: 1, rows: 7 }
        );
        assert_eq!(
            plan_of(&plan, 2, 1),
            DmaCompactChunkPlan { skip_ops: 1, count_ops: 1, rows: 1 }
        );
        assert_eq!(plan[0].1.rows, 8);
        assert_eq!(plan[1].1.rows, 10);
        assert_eq!(plan[2].1.rows, 1);
    }

    #[test]
    fn empty_plan() {
        let builder = DmaCompactInstancesBuilder::new(10);
        assert!(builder.get_plan().is_empty());
    }
}
