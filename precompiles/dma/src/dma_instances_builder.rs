use core::panic;
use std::collections::HashMap;

use zisk_common::{CheckPoint, ChunkId, CollectCounter};

use crate::{
    DmaCheckPoint, DmaCollectCounters, DmaInstanceInfo, DMA_COUNTER_INPUTCPY, DMA_COUNTER_MEMCMP,
    DMA_COUNTER_MEMCPY, DMA_COUNTER_MEMSET,
};
#[derive(Debug)]
pub struct DmaInstancesBuilder {
    pub tag: String,
    pub current_chunk: Option<ChunkId>,
    pub max_instances: usize,
    pub rows: usize,
    pub rows_available: usize,
    pub instances: Vec<DmaInstanceInfo>,
    pub count_memcpy_rows: usize,
    pub count_memset_rows: usize,
    pub count_memcmp_rows: usize,
    pub count_inputcpy_rows: usize,
    pub skip_memcpy_rows: usize,
    pub skip_memset_rows: usize,
    pub skip_memcmp_rows: usize,
    pub skip_inputcpy_rows: usize,
    pub inputs_counter: usize,
}

impl DmaInstancesBuilder {
    pub fn new(tag: &str, max_instances: usize, rows: usize) -> Self {
        Self {
            tag: tag.to_string(),
            current_chunk: None,
            max_instances,
            rows,
            rows_available: 0,
            instances: Vec::new(),
            skip_memcpy_rows: 0,
            skip_memset_rows: 0,
            skip_memcmp_rows: 0,
            skip_inputcpy_rows: 0,
            count_memcpy_rows: 0,
            count_memset_rows: 0,
            count_memcmp_rows: 0,
            count_inputcpy_rows: 0,
            inputs_counter: 0,
        }
    }
    pub fn count_to_skip(&mut self) {
        self.skip_memcpy_rows += self.count_memcpy_rows;
        self.skip_memset_rows += self.count_memset_rows;
        self.skip_memcmp_rows += self.count_memcmp_rows;
        self.skip_inputcpy_rows += self.count_inputcpy_rows;
        self.count_memcpy_rows = 0;
        self.count_memset_rows = 0;
        self.count_memcmp_rows = 0;
        self.count_inputcpy_rows = 0;
    }
    pub fn reset_count_and_skip(&mut self) {
        self.skip_memcpy_rows = 0;
        self.skip_memset_rows = 0;
        self.skip_memcmp_rows = 0;
        self.skip_inputcpy_rows = 0;
        self.count_memcpy_rows = 0;
        self.count_memset_rows = 0;
        self.count_memcmp_rows = 0;
        self.count_inputcpy_rows = 0;
    }

    pub fn open_new_instance(&mut self) {
        if self.rows_available > 0 {
            panic!(
                "[{}] Cannot open new instance, rows still available: {}",
                self.tag, self.rows_available
            );
        }
        if self.instances.len() >= self.max_instances {
            println!("{:?}", self);
            panic!(
                "[{}] Too many instances {} max: {}, cannot create more",
                self.tag,
                self.instances.len(),
                self.max_instances
            );
        }
        self.instances.push(DmaInstanceInfo { chunks: HashMap::new(), last_chunk: None });
        self.rows_available = self.rows;
    }
    pub fn flush_current_chunk(&mut self) {
        if let Some(chunk_id) = self.current_chunk {
            if self.count_memcpy_rows == 0
                && self.count_inputcpy_rows == 0
                && self.count_memset_rows == 0
                && self.count_memcmp_rows == 0
            {
                return;
            }
            if self.instances.is_empty() {
                self.open_new_instance();
            }
            let collect_counters = DmaCollectCounters {
                memcpy: CollectCounter::new(
                    self.skip_memcpy_rows as u32,
                    self.count_memcpy_rows as u32,
                ),
                inputcpy: CollectCounter::new(
                    self.skip_inputcpy_rows as u32,
                    self.count_inputcpy_rows as u32,
                ),
                memset: CollectCounter::new(
                    self.skip_memset_rows as u32,
                    self.count_memset_rows as u32,
                ),
                memcmp: CollectCounter::new(
                    self.skip_memcmp_rows as u32,
                    self.count_memcmp_rows as u32,
                ),
            };
            self.instances
                .last_mut()
                .unwrap()
                .chunks
                .insert(chunk_id, (self.inputs_counter as u64, collect_counters));
            self.instances.last_mut().unwrap().last_chunk = Some(chunk_id);
        }
    }
    #[inline(always)]
    pub fn add_memcpy_rows(&mut self, chunk_id: ChunkId, skip: usize, rows: usize, inputs: usize) {
        self.add_op_rows(chunk_id, skip, rows, inputs, DMA_COUNTER_MEMCPY);
    }
    #[inline(always)]
    pub fn add_memset_rows(&mut self, chunk_id: ChunkId, skip: usize, rows: usize, inputs: usize) {
        self.add_op_rows(chunk_id, skip, rows, inputs, DMA_COUNTER_MEMSET);
    }
    #[inline(always)]
    pub fn add_memcmp_rows(&mut self, chunk_id: ChunkId, skip: usize, rows: usize, inputs: usize) {
        self.add_op_rows(chunk_id, skip, rows, inputs, DMA_COUNTER_MEMCMP);
    }
    #[inline(always)]
    pub fn add_inputcpy_rows(
        &mut self,
        chunk_id: ChunkId,
        skip: usize,
        rows: usize,
        inputs: usize,
    ) {
        self.add_op_rows(chunk_id, skip, rows, inputs, DMA_COUNTER_INPUTCPY);
    }

    pub fn add_op_rows(
        &mut self,
        chunk_id: ChunkId,
        skip: usize,
        rows: usize,
        inputs: usize,
        op: usize,
    ) {
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
            let rows_applicable = std::cmp::min(self.rows_available, rows);
            rows -= rows_applicable;
            self.rows_available -= rows_applicable;
            match op {
                DMA_COUNTER_MEMCPY => {
                    if skip > 0 {
                        assert!(
                            self.count_memcpy_rows == 0,
                            "Cannot have both skip and count for memcpy in the same chunk",
                        );
                        self.skip_memcpy_rows += skip;
                    }
                    self.count_memcpy_rows += rows_applicable;
                }
                DMA_COUNTER_MEMSET => {
                    if skip > 0 {
                        assert!(
                            self.count_memset_rows == 0,
                            "Cannot have both skip and count for memset in the same chunk",
                        );
                        self.skip_memset_rows += skip;
                    }
                    self.count_memset_rows += rows_applicable;
                }
                DMA_COUNTER_MEMCMP => {
                    if skip > 0 {
                        assert!(
                            self.count_memcmp_rows == 0,
                            "Cannot have both skip and count for memcmp in the same chunk",
                        );
                        self.skip_memcmp_rows += skip;
                    }
                    self.count_memcmp_rows += rows_applicable;
                }
                DMA_COUNTER_INPUTCPY => {
                    if skip > 0 {
                        assert!(
                            self.count_inputcpy_rows == 0,
                            "Cannot have both skip and count for inputcpy in the same chunk",
                        );
                        self.skip_inputcpy_rows += skip;
                    }
                    self.count_inputcpy_rows += rows_applicable;
                }
                _ => {
                    panic!("Unsupported operation for DMA instance builder 0x{op:02X}")
                }
            }
            self.inputs_counter += inputs;
        }
    }
    pub fn flush(&mut self) {
        self.flush_current_chunk();
        self.reset_count_and_skip();
        self.current_chunk = None;
    }
    pub fn get_plan(&mut self) -> Vec<(CheckPoint, DmaCheckPoint)> {
        self.flush();
        let mut checkpoints = Vec::new();
        let last_segment_id = self.instances.len().saturating_sub(1);
        for (segment_id, dma_info) in self.instances.iter_mut().enumerate() {
            let keys = dma_info.chunks.keys().cloned().collect::<Vec<_>>();

            checkpoints.push((
                CheckPoint::Multiple(keys),
                DmaCheckPoint {
                    chunks: std::mem::take(&mut dma_info.chunks),
                    last_chunk: dma_info.last_chunk,
                    is_last_segment: segment_id == last_segment_id,
                },
            ));
        }
        checkpoints
    }
}
