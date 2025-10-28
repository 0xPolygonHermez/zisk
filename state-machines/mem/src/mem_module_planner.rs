use std::{collections::HashMap, sync::Arc};

use crate::{MemCountersCursor, MemPlanCalculator};
use mem_common::{MemCounters, MemModuleCheckPoint, MemModuleSegmentCheckPoint};
use std::cmp::min;
use zisk_common::{CheckPoint, ChunkId, InstanceType, Plan, SegmentId};
pub struct MemModulePlanner {
    config: MemModulePlannerConfig,
    rows_available: u32,
    last_addr: u32, // addr of last addr uses

    segments: Vec<MemModuleSegmentCheckPoint>,
    current_segment_chunks: HashMap<ChunkId, MemModuleCheckPoint>,

    first_chunk_id: Option<ChunkId>, // first chunk in segment, used for open segment
    last_chunk: Option<ChunkId>,
    current_chunk_id: Option<ChunkId>,
    reference_addr_chunk: Option<ChunkId>,
    reference_addr: u32,
    reference_skip: u32,
    cursor: MemCountersCursor,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MemModulePlannerConfig {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub addr_index: usize,
    pub from_addr: u32,
    pub last_addr: u32,
    pub rows: u32,
    pub consecutive_addr: bool,
}
impl MemModulePlanner {
    pub fn new(
        config: MemModulePlannerConfig,
        counters: Arc<Vec<(ChunkId, &MemCounters)>>,
    ) -> Self {
        Self {
            config,
            last_addr: config.last_addr,
            // first chunk is open
            rows_available: config.rows,
            segments: Vec::new(),
            current_chunk_id: None,
            current_segment_chunks: HashMap::new(),
            reference_addr_chunk: None,
            reference_addr: config.from_addr,
            reference_skip: 0,
            last_chunk: None,
            cursor: MemCountersCursor::new(counters, config.addr_index),
            first_chunk_id: None,
        }
    }
    pub fn module_plan(&mut self) {
        if self.cursor.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }
        // create a list of cursors, this list has the non-empty indexs of metric (couters) and his
        self.cursor.init();
        while !self.cursor.end() {
            // searches for the first smallest element in the vector
            let (chunk_id, addr, count) = self.cursor.get_next();
            self.add_to_current_instance(chunk_id, addr, count);
        }
        self.close_last_segment();
    }
    fn close_last_segment(&mut self) {
        if self.rows_available < self.config.rows {
            // close the last segment
            self.close_segment(true);
        } else if let Some(last_segment) = self.segments.last_mut() {
            last_segment.is_last_segment = true;
        }
    }

    /// Add "counter-address" to the current instance
    /// If the chunk_id is not in the list, it will be added. This method need to verify the
    /// distance between the last addr-step and the current addr-step, if the distance is
    /// greater than MEMORY_MAX_DIFF we need to add extra intermediate "steps".
    fn add_to_current_instance(&mut self, chunk_id: ChunkId, addr: u32, count: u32) {
        self.set_current_chunk_id(chunk_id);
        let intermediate_rows = self.add_intermediates(addr);
        self.preopen_segment(addr, intermediate_rows);
        self.set_reference(chunk_id, addr);
        self.add_rows(addr, count);
    }
    fn set_reference(&mut self, chunk_id: ChunkId, addr: u32) {
        self.reference_addr_chunk = Some(chunk_id);
        self.reference_addr = addr;
        self.reference_skip = 0;
    }

    fn set_current_chunk_id(&mut self, chunk_id: ChunkId) {
        self.last_chunk = self.current_chunk_id;
        self.current_chunk_id = Some(chunk_id);
    }
    fn close_segment(&mut self, is_last_segment: bool) {
        let chunks = std::mem::take(&mut self.current_segment_chunks);
        self.segments.push(MemModuleSegmentCheckPoint {
            chunks,
            is_last_segment,
            first_chunk_id: self.first_chunk_id,
        });
    }

    fn open_segment(&mut self) {
        // open a segment, must be set the reference chunk;
        // let segment_id = self.segments.len();
        self.close_segment(false);
        if let Some(reference_chunk) = self.reference_addr_chunk {
            // not use skip, because we use accumulated skip over reference, after open this segment,
            // add a block, if this block is the same chunk, local skips is ignored
            self.current_segment_chunks.insert(
                reference_chunk,
                MemModuleCheckPoint::new(self.reference_addr, self.reference_skip, 0),
            );
            self.first_chunk_id = Some(reference_chunk);
        }

        // all rows are available
        self.rows_available = self.config.rows;
    }
    fn add_next_addr_to_segment(&mut self, addr: u32) {
        let chunk_id = self.current_chunk_id.unwrap();
        self.add_chunk_to_segment(chunk_id, addr, 1, 0);
    }

    fn add_chunk_to_segment(&mut self, chunk_id: ChunkId, addr: u32, count: u32, skip: u32) {
        if self.current_segment_chunks.is_empty() {
            self.first_chunk_id = Some(chunk_id);
        }
        self.current_segment_chunks
            .entry(chunk_id)
            .and_modify(|checkpoint| checkpoint.add_rows(addr, count))
            .or_insert(MemModuleCheckPoint::new(addr, skip, count));
    }
    fn preopen_segment(&mut self, addr: u32, intermediate_rows: u32) {
        if self.rows_available == 0 {
            if intermediate_rows > 0 {
                // prevent last intermediate row zero
                self.add_next_addr_to_segment(addr);
            }
            self.open_segment();
        }
    }
    fn consume_rows(&mut self, addr: u32, rows: u32, skip: u32) {
        if rows == 0 && self.rows_available > 0 {
            return;
        }
        if rows > self.rows_available {
            panic!("MemModulePlanner::consume {}, too much rows {}", rows, self.rows_available);
        }

        // at this point we have a valid chunk_id
        let chunk_id = self.current_chunk_id.unwrap();

        if self.rows_available == 0 {
            self.open_segment();
        }

        self.add_chunk_to_segment(chunk_id, addr, rows, skip);
        self.rows_available -= rows;
        self.reference_skip += rows;
    }
    fn consume_intermediate_rows(&mut self, addr: u32, rows: u32, skip: u32) {
        if rows == 0 && self.rows_available > 0 {
            return;
        }
        if rows > self.rows_available {
            panic!("MemModulePlanner::consume {}, too much rows {}", rows, self.rows_available);
        }

        // at this point we have a valid chunk_id
        let chunk_id = self.current_chunk_id.unwrap();

        if self.rows_available == 0 {
            self.open_segment();
        }
        self.add_chunk_to_segment(chunk_id, addr, rows, skip);
        self.rows_available -= rows;
    }

    fn add_intermediate_rows(&mut self, addr: u32, count: u32) {
        let mut pending = count;

        while pending > 0 {
            let rows = min(pending, self.rows_available);
            let skip = count - pending;
            self.consume_intermediate_rows(addr, rows, skip);
            pending -= rows;
        }
    }

    fn add_rows(&mut self, addr: u32, count: u32) {
        let mut pending = count;
        while pending > 0 {
            let rows = min(pending, self.rows_available);
            let skip = count - pending;
            self.consume_rows(addr, rows, skip);
            pending -= rows;
        }
    }
    fn add_intermediate_addr(&mut self, from_addr: u32, to_addr: u32) {
        // adding internal reads of zero for consecutive addresses
        let count = to_addr - from_addr + 1;
        if count > 1 {
            self.add_intermediate_rows(from_addr, 1);
            self.add_intermediate_rows(to_addr, count - 1);
        } else {
            assert_eq!(to_addr, from_addr);
            self.add_intermediate_rows(to_addr, 1);
        }
    }
    fn add_intermediates(&mut self, addr: u32) -> u32 {
        if self.last_addr != addr {
            if self.config.consecutive_addr && (addr - self.last_addr) > 1 {
                self.add_intermediate_addr(self.last_addr + 1, addr - 1);
            }
            self.last_addr = addr;
        }
        0
    }
}

impl MemPlanCalculator for MemModulePlanner {
    fn plan(&mut self) {
        self.module_plan();
    }
    fn collect_plans(&mut self) -> Vec<Plan> {
        let mut plans: Vec<Plan> = Vec::new();
        if self.segments.is_empty() {
            // no data => no plans
            return plans;
        }

        let segments = std::mem::take(&mut self.segments);
        for (segment_id, segment) in segments.into_iter().enumerate() {
            let keys = segment.chunks.keys().cloned().collect::<Vec<_>>();
            plans.push(Plan::new(
                self.config.airgroup_id,
                self.config.air_id,
                Some(SegmentId(segment_id)),
                InstanceType::Instance,
                CheckPoint::Multiple(keys),
                Some(Box::new(segment)),
            ));
        }
        plans
    }
}
