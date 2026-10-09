use std::collections::HashMap;

use zisk_common::{CheckPoint, ChunkId, CollectCounter};

use crate::{
    DmaWithPrePostCollectCounters, DMA_WPP_CLASSES, DMA_WPP_CLASS_DOUBLE, DMA_WPP_CLASS_ROWS,
    DMA_WPP_CLASS_SINGLE,
};

/// What one `DmaWithPrePost` instance has to collect, per chunk.
#[derive(Default, Debug)]
pub struct DmaWithPrePostCheckPoint {
    /// Per chunk: the upper bound on operations the collector keeps, and which of them.
    pub chunks: HashMap<ChunkId, (u64, DmaWithPrePostCollectCounters)>,
    pub last_chunk: Option<ChunkId>,
    pub is_last_segment: bool,
}

impl DmaWithPrePostCheckPoint {
    #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_plans"))]
    pub fn get_debug_info(&self, title: &str, segment_id: u64) -> String {
        self.chunks
            .iter()
            .map(|(chunk_id, (num_ops, collect_counters))| {
                format!(
                    "{title} #{segment_id}@{chunk_id} [{num_ops}|{}]{}{}",
                    collect_counters.get_debug_info(),
                    if Some(*chunk_id) == self.last_chunk { " [last_chunk]" } else { "" },
                    if self.is_last_segment { " [last_segment]" } else { "" },
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug)]
struct InstanceInfo {
    chunks: HashMap<ChunkId, (u64, DmaWithPrePostCollectCounters)>,
    last_chunk: Option<ChunkId>,
}

/// Hands the DMA operations of every chunk out to `DmaWithPrePost` instances.
///
/// The difference with [`crate::DmaInstancesBuilder`] is that here an **operation is atomic**: a
/// two-row operation puts its PRE row right after its DMA row, so it cannot be cut between two
/// instances. When only one row is left and the next operation needs two, that last row is dropped
/// as padding and the operation starts the following instance — at most one wasted row per
/// instance, which is what [`Self::instances_needed`] budgets for.
#[derive(Debug)]
pub struct DmaWithPrePostInstancesBuilder {
    pub tag: String,
    pub current_chunk: Option<ChunkId>,
    pub max_instances: usize,
    pub rows: usize,
    pub rows_available: usize,
    instances: Vec<InstanceInfo>,
    /// Operations of each class recorded for the (instance, chunk) being filled.
    count_ops: [usize; DMA_WPP_CLASSES],
    /// Operations of each class of the current chunk already taken by previous instances.
    skip_ops: [usize; DMA_WPP_CLASSES],
}

impl DmaWithPrePostInstancesBuilder {
    pub fn new(tag: &str, max_instances: usize, rows: usize) -> Self {
        Self {
            tag: tag.to_string(),
            current_chunk: None,
            max_instances,
            rows,
            rows_available: 0,
            instances: Vec::new(),
            count_ops: [0; DMA_WPP_CLASSES],
            skip_ops: [0; DMA_WPP_CLASSES],
        }
    }

    /// Instances needed to prove `rows` rows. Every instance is guaranteed to take at least
    /// `rows - 1` of them: the only row that can be left unused is the last one of an instance,
    /// when the next operation needs two rows.
    pub fn instances_needed(total_rows: usize, rows: usize) -> usize {
        assert!(rows >= DMA_WPP_CLASS_ROWS[DMA_WPP_CLASS_DOUBLE]);
        total_rows.div_ceil(rows - 1)
    }

    fn count_to_skip(&mut self) {
        for class in 0..DMA_WPP_CLASSES {
            self.skip_ops[class] += self.count_ops[class];
            self.count_ops[class] = 0;
        }
    }

    fn reset_count_and_skip(&mut self) {
        self.count_ops = [0; DMA_WPP_CLASSES];
        self.skip_ops = [0; DMA_WPP_CLASSES];
    }

    fn open_new_instance(&mut self) {
        assert_eq!(
            self.rows_available, 0,
            "[{}] Cannot open new instance, rows still available: {}",
            self.tag, self.rows_available
        );
        assert!(
            self.instances.len() < self.max_instances,
            "[{}] Too many instances {} max: {}, cannot create more",
            self.tag,
            self.instances.len(),
            self.max_instances
        );
        self.instances.push(InstanceInfo { chunks: HashMap::new(), last_chunk: None });
        self.rows_available = self.rows;
    }

    fn flush_current_chunk(&mut self) {
        let Some(chunk_id) = self.current_chunk else {
            return;
        };
        let num_ops: usize = self.count_ops.iter().sum();
        if num_ops == 0 {
            return;
        }
        if self.instances.is_empty() {
            self.open_new_instance();
        }
        let collect_counters = DmaWithPrePostCollectCounters::new(
            CollectCounter::new(
                self.skip_ops[DMA_WPP_CLASS_SINGLE] as u32,
                self.count_ops[DMA_WPP_CLASS_SINGLE] as u32,
            ),
            CollectCounter::new(
                self.skip_ops[DMA_WPP_CLASS_DOUBLE] as u32,
                self.count_ops[DMA_WPP_CLASS_DOUBLE] as u32,
            ),
        );
        let instance = self.instances.last_mut().unwrap();
        instance.chunks.insert(chunk_id, (num_ops as u64, collect_counters));
        instance.last_chunk = Some(chunk_id);
    }

    /// Adds `ops` operations of `class` from `chunk_id` to the instance currently being filled.
    pub fn add_ops(&mut self, chunk_id: ChunkId, class: usize, ops: usize) {
        if ops == 0 {
            return;
        }
        if Some(chunk_id) != self.current_chunk {
            self.flush_current_chunk();
            self.reset_count_and_skip();
            self.current_chunk = Some(chunk_id);
        }
        let class_rows = DMA_WPP_CLASS_ROWS[class];
        let mut ops = ops;
        while ops > 0 {
            let fit = self.rows_available / class_rows;
            if fit == 0 {
                // Not one more operation of this class fits. The rows still available (at most
                // one, and only when this class needs two) are dropped as padding: an operation
                // cannot be split, its PRE row must stay next to its DMA row.
                self.flush_current_chunk();
                self.count_to_skip();
                self.rows_available = 0;
                self.open_new_instance();
                continue;
            }
            let applicable = fit.min(ops);
            ops -= applicable;
            self.rows_available -= applicable * class_rows;
            self.count_ops[class] += applicable;
        }
    }

    pub fn flush(&mut self) {
        self.flush_current_chunk();
        self.reset_count_and_skip();
        self.current_chunk = None;
    }

    pub fn get_plan(&mut self) -> Vec<(CheckPoint, DmaWithPrePostCheckPoint)> {
        self.flush();
        let last_segment_id = self.instances.len().saturating_sub(1);
        self.instances
            .iter_mut()
            .enumerate()
            .map(|(segment_id, info)| {
                let keys = info.chunks.keys().cloned().collect::<Vec<_>>();
                (
                    CheckPoint::Multiple(keys),
                    DmaWithPrePostCheckPoint {
                        chunks: std::mem::take(&mut info.chunks),
                        last_chunk: info.last_chunk,
                        is_last_segment: segment_id == last_segment_id,
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "../tests/dma_with_pre_post_builder_tests.rs"]
mod tests;
