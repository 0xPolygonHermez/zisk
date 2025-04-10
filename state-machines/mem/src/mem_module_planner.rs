use std::sync::Arc;

use crate::{
    MemCounters, MemCountersCursor, MemHelpers, MemPlanCalculator, CHUNK_MAX_DISTANCE,
    STEP_MEMORY_MAX_DIFF,
};
use sm_common::{CheckPoint, ChunkId, InstanceType, Plan};
use std::cmp::min;

#[derive(Debug, Default, Clone)]
pub struct MemModuleSegmentCheckPoint {
    pub reference_addr: u32,
    pub last_addr: u32,
    pub skip_rows: u32,
    pub rows: u32,
    pub is_last_segment: bool,
    pub last_addr_chunk: ChunkId,
    pub reference_addr_chunk: ChunkId,
}

pub struct MemModulePlanner {
    config: MemModulePlannerConfig,
    rows_available: u32,
    last_addr_chunk: ChunkId,
    last_addr: u32, // addr of last addr uses

    segments: Vec<MemModuleSegmentCheckPoint>,
    segment_chunks: Vec<Vec<ChunkId>>,
    last_chunk_id: Option<ChunkId>,
    current_chunk_id: Option<ChunkId>,
    reference_addr_chunk: Option<ChunkId>,
    reference_addr: u32,
    reference_skip: u32,
    last_chunk_id_inserted: Option<ChunkId>,
    cursor: MemCountersCursor,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MemModulePlannerConfig {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub addr_index: usize,
    pub from_addr: u32,
    pub rows: u32,
    pub consecutive_addr: bool,
    pub intermediate_step_reads: bool,
}
impl<'a> MemModulePlanner {
    pub fn new(
        config: MemModulePlannerConfig,
        counters: Arc<Vec<(ChunkId, &MemCounters)>>,
    ) -> Self {
        Self {
            config,
            last_addr: config.from_addr,
            last_addr_chunk: 0,
            rows_available: 0,
            segments: Vec::new(),
            segment_chunks: Vec::new(),
            current_chunk_id: None,
            reference_addr_chunk: None,
            reference_addr: config.from_addr,
            reference_skip: 0,
            last_chunk_id: None,
            last_chunk_id_inserted: None,
            cursor: MemCountersCursor::new(counters, config.addr_index),
        }
    }
    pub fn module_plan(&mut self) {
        if self.cursor.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }
        // create a list of cursors, this list has the non-empty indexs of metric (couters) and his
        self.cursor.init();
        while !self.cursor.end() {
            println!("[Mem] cursor1");
            // searches for the first smallest element in the vector
            let (chunk_id, addr, count) = self.cursor.get_next();
            println!("[Mem] cursor2");
            self.add_to_current_instance(chunk_id, addr, count);
            println!("[Mem] cursor3");
        }
        self.update_last_segment_id();
    }
    fn update_last_segment_id(&mut self) {
        if let Some(last_segment) = self.segments.last_mut() {
            last_segment.is_last_segment = true;
        }
    }

    /// Add "counter-address" to the current instance
    /// If the chunk_id is not in the list, it will be added. This method need to verify the
    /// distance between the last addr-step and the current addr-step, if the distance is
    /// greater than MEMORY_MAX_DIFF we need to add extra intermediate "steps".
    fn add_to_current_instance(&mut self, chunk_id: ChunkId, addr: u32, count: u32) {
        self.set_current_chunk_id(chunk_id);
        self.add_intermediate_rows(addr);
        self.preopen_segment();
        self.set_reference(chunk_id, addr);
        self.add_rows(count);
    }
    fn set_reference(&mut self, chunk_id: ChunkId, addr: u32) {
        self.reference_addr_chunk = Some(chunk_id);
        self.reference_addr = addr;
        self.reference_skip = 0;
    }

    fn set_current_chunk_id(&mut self, chunk_id: ChunkId) {
        self.last_chunk_id = self.current_chunk_id;
        self.current_chunk_id = Some(chunk_id);
    }
    fn open_segment(&mut self) {
        self.segments.push(MemModuleSegmentCheckPoint {
            reference_addr: self.reference_addr,
            // TODO: helper memory to calculate first step
            // reference_step: self.reference_chunk_id * CHUNK_SIZE as u64,
            reference_addr_chunk: 0,
            skip_rows: self.reference_skip,
            rows: 0,
            is_last_segment: false,
            last_addr: self.last_addr,
            last_addr_chunk: self.last_addr_chunk,
        });
        let chunks = if let Some(id) = self.reference_addr_chunk { vec![id] } else { Vec::new() };
        self.segment_chunks.push(chunks);
        // to cache last chunk id
        self.last_chunk_id_inserted = self.reference_addr_chunk;
        // all rows are available
        self.rows_available = self.config.rows;
    }
    fn add_chunk_to_segment(&mut self, segment_id: usize, chunk_id: ChunkId) {
        if Some(chunk_id) != self.last_chunk_id_inserted {
            if let Err(pos) = self.segment_chunks[segment_id].binary_search(&chunk_id) {
                self.segment_chunks[segment_id].insert(pos, chunk_id);
            }
            self.last_chunk_id_inserted = Some(chunk_id);
        }
    }
    fn preopen_segment(&mut self) {
        if self.rows_available == 0 {
            self.open_segment();
        }
    }
    fn consume_rows(&mut self, rows: u32) {
        if rows == 0 && self.rows_available > 0 {
            return;
        }
        if rows > self.rows_available {
            panic!("MemModulePlanner::consume {}, too much rows {}", rows, self.rows_available);
        }

        let chunk_id = self.current_chunk_id.unwrap();

        if self.rows_available == 0 {
            self.open_segment();
        }

        let segment_id = self.segments.len() - 1;

        // TODO: open segment();
        if self.segments[segment_id].rows == 0 {
            self.add_chunk_to_segment(segment_id, chunk_id);
        }

        self.segments[segment_id].rows += rows;
        self.rows_available -= rows;
        self.reference_skip += rows;
    }
    fn add_rows(&mut self, count: u32) {
        // check if all internal reads fit in the current instance
        let mut pending = count;
        println!("add_rows: {} available: {}", count, self.rows_available);
        while pending > 0 {
            // if pending <= self.rows_available {
            let rows = min(pending, self.rows_available);
            println!("min(pending:{},available:{})={}", pending, self.rows_available, rows);
            self.consume_rows(rows);
            pending -= rows;
        }
    }
    fn add_intermediate_rows(&mut self, addr: u32) {
        if self.last_addr != addr {
            if self.config.consecutive_addr && addr - self.last_addr > 1 {
                // adding internal reads of zero for consecutive addresses
                println!(
                    "\x1B[1;33m[Mem] add_intermediate_rows(addr): 0x{:X} - 0x{:X} = {}\x1B[0m",
                    8 * addr,
                    8 * self.last_addr,
                    addr - self.last_addr - 1
                );
                self.add_rows(addr - self.last_addr - 1);
            }
            self.last_addr = addr;
            return;
        }

        if !self.config.intermediate_step_reads {
            return;
        }

        // check if the distance between the last chunk and the current is too large, if so then
        // we need to add intermediate rows
        if let Some(last_chunk_id) = self.last_chunk_id {
            let chunk = self.current_chunk_id.unwrap();
            let chunk_distance = chunk - last_chunk_id;
            if chunk_distance > CHUNK_MAX_DISTANCE {
                let distance = MemHelpers::max_distance_between_chunks(last_chunk_id, chunk);
                let intermediate_rows = distance / STEP_MEMORY_MAX_DIFF;
                if intermediate_rows > 0 {
                    println!(
                        "\x1B[1;33m[Mem] add_intermediate_rows(steps): 0x{:X} #:{}\x1B[0m",
                        addr * 8,
                        intermediate_rows
                    );
                    self.add_rows(intermediate_rows as u32);
                }
            }
        }
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

        let segments_count = self.segments.len();
        for segment_id in 0..segments_count {
            let chunks = std::mem::take(&mut self.segment_chunks[segment_id]);
            let checkpoint = std::mem::take(&mut self.segments[segment_id]);
            println!("[Mem] checkpoint({}): {:?}", segment_id, checkpoint);
            plans.push(Plan::new(
                self.config.airgroup_id,
                self.config.air_id,
                Some(SegmentId(segment_id)),
                InstanceType::Instance,
                CheckPoint::Multiple(chunks),
                Some(Box::new(checkpoint)),
            ));
        }
        plans
    }
}
