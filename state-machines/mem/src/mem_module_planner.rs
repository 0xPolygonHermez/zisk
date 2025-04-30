use std::{collections::HashMap, sync::Arc};

use crate::{
    MemCounters, MemCountersCursor, MemHelpers, MemModuleCheckPoint, MemPlanCalculator,
    CHUNK_MAX_DISTANCE, STEP_MEMORY_MAX_DIFF,
};
use sm_common::{CheckPoint, InstanceType, Plan};
use std::cmp::min;
use zisk_common::{ChunkId, SegmentId};

#[derive(Debug, Default, Clone)]
pub struct MemModuleSegmentCheckPoint {
    pub chunks: HashMap<ChunkId, MemModuleCheckPoint>,
    pub is_last_segment: bool,
}

impl MemModuleSegmentCheckPoint {
    #[allow(dead_code)]
    fn to_string(&self, segment_id: usize) -> String {
        let mut result = String::new();
        for (chunk_id, checkpoint) in &self.chunks {
            result = result
                + &format!(
                    "#{}@{}  [0x{:08X} s:{}], [0x{:08X} C:{}] C:{} intermediate_skip:{:?}\n",
                    segment_id,
                    chunk_id,
                    checkpoint.from_addr * 8,
                    checkpoint.from_skip,
                    checkpoint.to_addr * 8,
                    checkpoint.to_count,
                    checkpoint.count,
                    checkpoint.intermediate_skip
                );
        }
        result
    }
}

pub struct MemModulePlanner {
    config: MemModulePlannerConfig,
    rows_available: u32,
    last_addr: u32, // addr of last addr uses

    segments: Vec<MemModuleSegmentCheckPoint>,
    current_segment_chunks: HashMap<ChunkId, MemModuleCheckPoint>,

    last_chunk: Option<ChunkId>,
    current_chunk_id: Option<ChunkId>,
    reference_addr_chunk: Option<ChunkId>,
    reference_addr: u32,
    reference_skip: u32,
    cursor: MemCountersCursor,
    intermediate_extra_rows: u64,
    intermediate_count: u64,
    intermediate_max: u64,
    intermediate_max_count: u64,
    intermediate_rows: u64,
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
            intermediate_extra_rows: 0,
            intermediate_rows: 0,
            intermediate_count: 0,
            intermediate_max: 0,
            intermediate_max_count: 0,
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
            // println!("COUNTER: 0x{:X} CHUNK: {} COUNT: {}", addr * 8, chunk_id, count);
            self.add_to_current_instance(chunk_id, addr, count);
        }
        self.close_last_segment();
        // log::info!(
        //     "MemPlan : ··· Intermediate rows[{}:{}] 0x{:X} => {} {} {} {} {} (rows,#,extra,max,#max)",
        //     self.config.airgroup_id,
        //     self.config.air_id,
        //     self.config.from_addr * 8,
        //     self.intermediate_rows,
        //     self.intermediate_count,
        //     self.intermediate_extra_rows,
        //     self.intermediate_max,
        //     self.intermediate_max_count
        // );
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
        self.segments.push(MemModuleSegmentCheckPoint { chunks, is_last_segment });
    }

    fn open_segment(&mut self, intermediate_skip: Option<u32>) {
        // open a segment, must be set the reference chunk;
        // let segment_id = self.segments.len();
        self.close_segment(false);
        if let Some(reference_chunk) = self.reference_addr_chunk {
            // not use skip, because we use accumulated skip over reference, after open this segment,
            // add a block, if this block is the same chunk, local skips is ignored
            // println!(
            //     "OPEN SEGMENT #{}: REFERENCE 0x{:X}+C:{}+{} {:?}",
            //     segment_id,
            //     self.reference_addr * 8,
            //     reference_chunk,
            //     self.reference_skip,
            //     intermediate_skip
            // );
            self.current_segment_chunks.insert(
                reference_chunk,
                MemModuleCheckPoint::new(
                    self.reference_addr,
                    self.reference_skip,
                    0,
                    intermediate_skip,
                ),
            );
        } else {
            // println!("OPEN SEGMENT #{}: NO_REFERENCE", segment_id);
        }

        // all rows are available
        self.rows_available = self.config.rows;
    }
    fn add_next_addr_to_segment(&mut self, addr: u32) {
        let chunk_id = self.current_chunk_id.unwrap();
        // println!(
        //     "ADDING NEXT ADDR TO SEGMENT #{}: 0x{:X} C:{}",
        //     self.segments.len(),
        //     addr * 8,
        //     chunk_id
        // );
        self.add_chunk_to_segment(chunk_id, addr, 1, 0);
    }

    fn add_chunk_to_segment(&mut self, chunk_id: ChunkId, addr: u32, count: u32, skip: u32) {
        // if addr >= 268435456 && addr <= 301989880 {
        //     println!(
        //         "ADD CHUNK #{}: 0x{:X} C:{}+{} 0x{:X}+C:{}+{} SKIP:{}",
        //         self.segments.len(),
        //         addr * 8,
        //         chunk_id,
        //         count,
        //         self.reference_addr * 8,
        //         self.reference_addr_chunk.unwrap_or_default(),
        //         self.reference_skip,
        //         skip
        //     );
        // }
        self.current_segment_chunks
            .entry(chunk_id)
            .and_modify(|checkpoint| checkpoint.add_rows(addr, count))
            .or_insert(MemModuleCheckPoint::new(addr, skip, count, None));
    }
    fn preopen_segment(&mut self, addr: u32, intermediate_rows: u32) {
        if self.rows_available == 0 {
            if intermediate_rows > 0 {
                // prevent last intermediate row zero
                self.add_next_addr_to_segment(addr);
            }
            self.open_segment(Some(intermediate_rows));
        }
    }
    fn consume_rows(&mut self, addr: u32, rows: u32, skip: u32) {
        // if addr >= 268435456 && addr <= 301989880 {
        //     let chunk = self.current_chunk_id.unwrap_or_default();
        //     let last_chunk = self.last_chunk.unwrap_or_default();
        //     println!(
        //         "CONSUME[{},{}] {} 0x{:X},C:{} LC:{} REF({}):0x{:X}+{} SKIP:{}",
        //         self.segments.len(),
        //         self.config.rows - self.rows_available,
        //         rows,
        //         addr * 8,
        //         chunk,
        //         last_chunk,
        //         self.reference_addr_chunk.unwrap_or_default(),
        //         self.reference_addr * 8,
        //         self.reference_skip,
        //         skip
        //     );
        // }
        if rows == 0 && self.rows_available > 0 {
            return;
        }
        if rows > self.rows_available {
            panic!("MemModulePlanner::consume {}, too much rows {}", rows, self.rows_available);
        }

        // at this point we have a valid chunk_id
        let chunk_id = self.current_chunk_id.unwrap();

        if self.rows_available == 0 {
            self.open_segment(Some(0));
        }

        self.add_chunk_to_segment(chunk_id, addr, rows, skip);
        self.rows_available -= rows;
        self.reference_skip += rows;
    }
    fn consume_intermediate_rows(&mut self, addr: u32, rows: u32, skip: u32) {
        // if addr >= 268435456 && addr <= 301989880 {
        //     let chunk = self.current_chunk_id.unwrap_or_default();
        //     let last_chunk = self.last_chunk.unwrap_or_default();
        //     println!(
        //         "CONSUME_INTERMEDIATE[{},{}] {} 0x{:X},C:{} LC:{} REF({}):0x{:X}+{} SKIP:{}",
        //         self.segments.len(),
        //         self.config.rows - self.rows_available,
        //         rows,
        //         addr * 8,
        //         chunk,
        //         last_chunk,
        //         self.reference_addr_chunk.unwrap_or_default(),
        //         self.reference_addr * 8,
        //         self.reference_skip,
        //         skip
        //     );
        // }

        if rows == 0 && self.rows_available > 0 {
            return;
        }
        if rows > self.rows_available {
            panic!("MemModulePlanner::consume {}, too much rows {}", rows, self.rows_available);
        }

        // at this point we have a valid chunk_id
        let chunk_id = self.current_chunk_id.unwrap();

        if self.rows_available == 0 {
            self.open_segment(Some(skip));
        }
        if !self.config.intermediate_step_reads {
            self.add_chunk_to_segment(chunk_id, addr, rows, skip);
        }
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
        // if addr >= 268435456 && addr <= 301989880 {
        //     let chunk = self.current_chunk_id.unwrap_or_default();
        //     let last_chunk = self.last_chunk.unwrap_or_default();
        //     println!(
        //         "ADD_ROWS[{},{}] {} 0x{:X},C:{} LC:{} REF({}):0x{:X}+{}",
        //         self.segments.len(),
        //         self.config.rows - self.rows_available,
        //         count,
        //         addr * 8,
        //         chunk,
        //         last_chunk,
        //         self.reference_addr_chunk.unwrap_or_default(),
        //         self.reference_addr * 8,
        //         self.reference_skip
        //     );
        // }

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
        self.intermediate_rows += count as u64;
        self.intermediate_count += 1;
        if count as u64 > self.intermediate_max {
            self.intermediate_max_count = 1;
            self.intermediate_max = count as u64;
        } else if count as u64 == self.intermediate_max {
            self.intermediate_max_count += 1;
        }
    }
    fn add_intermediates(&mut self, addr: u32) -> u32 {
        if self.last_addr != addr {
            if self.config.consecutive_addr && (addr - self.last_addr) > 1 {
                self.add_intermediate_addr(self.last_addr + 1, addr - 1);
            }
            self.last_addr = addr;
        } else if self.config.intermediate_step_reads {
            return self.add_intermediate_steps(addr);
        }
        0
    }
    fn add_intermediate_steps(&mut self, addr: u32) -> u32 {
        // check if the distance between the last chunk and the current is too large,
        // if so then we need to add intermediate rows
        let mut intermediate_rows = 0;
        if let Some(last_chunk) = self.last_chunk {
            let chunk = self.current_chunk_id.unwrap();
            let chunk_distance = chunk.0 - last_chunk.0;
            if chunk_distance > CHUNK_MAX_DISTANCE {
                let distance = MemHelpers::max_distance_between_chunks(last_chunk, chunk);
                intermediate_rows = (distance - 1) / STEP_MEMORY_MAX_DIFF;
                if intermediate_rows == 0 {
                    // self.intermediate_extra_rows += 1;
                    intermediate_rows = 1;
                }
                // let segment_id = self.segments.len();
                // if segment_id >= 52 || segment_id <= 54 {
                //     println!(
                //         "INTERMEDIATE_STEPS[{},{}] {} 0x{:X},C:{} LC:{} CD:{} D:{} REF({}):0x{:X}+{}",
                //         self.segments.len(),
                //         self.config.rows - self.rows_available,
                //         intermediate_rows,
                //         addr * 8,
                //         chunk,
                //         last_chunk,
                //         chunk_distance,
                //         distance,
                //         self.reference_addr_chunk.unwrap_or_default(),
                //         self.reference_addr * 8,
                //         self.reference_skip
                //     );
                // }
                self.add_intermediate_rows(addr, intermediate_rows as u32);
                // self.intermediate_rows += intermediate_rows;
                // self.intermediate_count += 1;
                // if intermediate_rows > self.intermediate_max {
                //     self.intermediate_max_count = 1;
                //     self.intermediate_max = intermediate_rows;
                // } else if intermediate_rows == self.intermediate_max {
                //     self.intermediate_max_count += 1;
                // }
            }
        }
        intermediate_rows as u32
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
            // for (ck_id, checkpoint) in &segment.chunks {
            //     if segment_id >= 52 && segment_id <= 55 {
            //         println!(
            //             "[{}:{},{}]: {} {:?}",
            //             self.config.airgroup_id, self.config.air_id, segment_id, ck_id, checkpoint
            //         );
            //     }
            // }
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
