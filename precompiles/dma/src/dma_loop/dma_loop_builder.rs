//! Handing the loop rows of every chunk out to the instances of a `DmaLoop` air (or of the loop
//! block of `CompactDma`).
//!
//! The same bookkeeping as [`crate::DmaInstancesBuilder`], with one counter per [`DMA_LOOP_CLASSES`]
//! class instead of one per opcode: the aligned and the unaligned loops of the same opcode may be
//! routed to different airs, so an instance has to know which of the two it takes.

use std::collections::HashMap;

use zisk_common::{CheckPoint, ChunkId, CollectCounter};

use crate::DMA_LOOP_CLASSES;

/// How many rows of each class an instance collects from a chunk.
#[derive(Debug, Clone, Copy)]
pub struct DmaLoopCollectCounters {
    pub classes: [CollectCounter; DMA_LOOP_CLASSES],
}

impl DmaLoopCollectCounters {
    /// A counter that collects nothing.
    pub fn empty() -> Self {
        Self { classes: [CollectCounter::new(0, 0); DMA_LOOP_CLASSES] }
    }

    /// `Some((skip, rows))` when the next operation of `class`, taking `rows` rows, belongs to this
    /// instance: the rows of it already proved elsewhere, and the rows this instance takes.
    #[inline(always)]
    pub fn should_collect(&mut self, class: usize, rows: u64) -> Option<(u32, u32)> {
        self.classes[class].should_process(rows as u32)
    }

    pub fn is_final_skip(&self) -> bool {
        self.classes.iter().all(|c| c.is_final_skip())
    }

    /// Rows this instance is budgeted in this chunk, over every class.
    pub fn total_collect_count(&self) -> u64 {
        self.classes.iter().map(|c| c.collect_count as u64).sum()
    }

    #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_plans"))]
    pub fn get_debug_info(&self) -> String {
        const NAMES: [&str; DMA_LOOP_CLASSES] = ["CY", "MS", "MC", "IC", "UCY", "UMC"];
        self.classes
            .iter()
            .zip(NAMES)
            .map(|(c, name)| {
                if c.initial_skip == 0 {
                    format!("{name}:{}", c.collect_count)
                } else {
                    format!("{name}:({}){}", c.collect_count, c.initial_skip)
                }
            })
            .collect::<Vec<_>>()
            .join("|")
    }
}

/// What one `DmaLoop` instance has to collect, per chunk.
#[derive(Default, Debug)]
pub struct DmaLoopCheckPoint {
    /// Per chunk: the upper bound on inputs the collector keeps, and which rows of each class.
    pub chunks: HashMap<ChunkId, (u64, DmaLoopCollectCounters)>,
    pub last_chunk: Option<ChunkId>,
    pub is_last_segment: bool,
}

impl DmaLoopCheckPoint {
    #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_plans"))]
    pub fn get_debug_info(&self, title: &str, segment_id: u64) -> String {
        self.chunks
            .iter()
            .map(|(chunk_id, (num_inputs, counters))| {
                format!(
                    "{title} #{segment_id}@{chunk_id} [{num_inputs}|{}]{}{}",
                    counters.get_debug_info(),
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
    chunks: HashMap<ChunkId, (u64, DmaLoopCollectCounters)>,
    last_chunk: Option<ChunkId>,
}

/// Hands the loop rows out to the instances of one air, chunk after chunk, cutting a sequence
/// wherever an instance runs out of rows: the loop airs keep a continuation chain, so a sequence
/// may be split between two instances.
#[derive(Debug)]
pub struct DmaLoopInstancesBuilder {
    pub tag: String,
    pub current_chunk: Option<ChunkId>,
    pub max_instances: usize,
    pub rows: usize,
    pub rows_available: usize,
    instances: Vec<InstanceInfo>,
    /// Rows of each class recorded for the (instance, chunk) being filled.
    count_rows: [usize; DMA_LOOP_CLASSES],
    /// Rows of each class of the current chunk already taken by previous instances.
    skip_rows: [usize; DMA_LOOP_CLASSES],
    /// Upper bound on the inputs the (instance, chunk) being filled collects.
    inputs_counter: usize,
}

impl DmaLoopInstancesBuilder {
    pub fn new(tag: &str, max_instances: usize, rows: usize) -> Self {
        Self {
            tag: tag.to_string(),
            current_chunk: None,
            max_instances,
            rows,
            rows_available: 0,
            instances: Vec::new(),
            count_rows: [0; DMA_LOOP_CLASSES],
            skip_rows: [0; DMA_LOOP_CLASSES],
            inputs_counter: 0,
        }
    }

    fn count_to_skip(&mut self) {
        for class in 0..DMA_LOOP_CLASSES {
            self.skip_rows[class] += self.count_rows[class];
            self.count_rows[class] = 0;
        }
    }

    fn reset_count_and_skip(&mut self) {
        self.count_rows = [0; DMA_LOOP_CLASSES];
        self.skip_rows = [0; DMA_LOOP_CLASSES];
    }

    fn open_new_instance(&mut self) {
        assert_eq!(
            self.rows_available, 0,
            "[{}] cannot open a new instance, {} rows still available",
            self.tag, self.rows_available
        );
        assert!(
            self.instances.len() < self.max_instances,
            "[{}] too many instances {}, max {}",
            self.tag,
            self.instances.len(),
            self.max_instances
        );
        self.instances.push(InstanceInfo { chunks: HashMap::new(), last_chunk: None });
        self.rows_available = self.rows;
    }

    fn flush_current_chunk(&mut self) {
        let Some(chunk_id) = self.current_chunk else { return };
        if self.count_rows.iter().all(|&rows| rows == 0) {
            return;
        }
        if self.instances.is_empty() {
            self.open_new_instance();
        }
        let counters = DmaLoopCollectCounters {
            classes: std::array::from_fn(|class| {
                CollectCounter::new(self.skip_rows[class] as u32, self.count_rows[class] as u32)
            }),
        };
        let instance = self.instances.last_mut().unwrap();
        instance.chunks.insert(chunk_id, (self.inputs_counter as u64, counters));
        instance.last_chunk = Some(chunk_id);
        // The bound belongs to the (instance, chunk) record just stored.
        self.inputs_counter = 0;
    }

    /// Adds `rows` rows of `class` from `chunk_id`, spread over `inputs` operations.
    pub fn add_class_rows(&mut self, chunk_id: ChunkId, class: usize, rows: usize, inputs: usize) {
        if rows == 0 {
            return;
        }
        if Some(chunk_id) != self.current_chunk {
            self.flush_current_chunk();
            self.reset_count_and_skip();
            self.current_chunk = Some(chunk_id);
        }
        let mut rows = rows;
        while rows > 0 {
            if self.rows_available == 0 {
                self.flush_current_chunk();
                self.count_to_skip();
                self.open_new_instance();
            }
            let applicable = self.rows_available.min(rows);
            rows -= applicable;
            self.rows_available -= applicable;
            self.count_rows[class] += applicable;
            // An operation straddling an instance boundary is collected (partially) by both, so
            // `inputs` bounds every instance the batch reaches.
            self.inputs_counter += inputs;
        }
    }

    pub fn flush(&mut self) {
        self.flush_current_chunk();
        self.reset_count_and_skip();
        self.current_chunk = None;
    }

    /// One checkpoint per instance, in segment order.
    pub fn get_plan(&mut self) -> Vec<(CheckPoint, DmaLoopCheckPoint)> {
        self.flush();
        let last_segment_id = self.instances.len().saturating_sub(1);
        self.instances
            .iter_mut()
            .enumerate()
            .map(|(segment_id, info)| {
                let mut keys = info.chunks.keys().cloned().collect::<Vec<_>>();
                keys.sort_unstable();
                (
                    CheckPoint::Multiple(keys),
                    DmaLoopCheckPoint {
                        chunks: std::mem::take(&mut info.chunks),
                        last_chunk: info.last_chunk,
                        is_last_segment: segment_id == last_segment_id,
                    },
                )
            })
            .collect()
    }
}
