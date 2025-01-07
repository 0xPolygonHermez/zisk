use std::sync::Arc;

use crate::{MemCounters, MemPlanCalculator};
use sm_common::{CheckPoint, ChunkId, CollectInfoSkip, InstanceType, Plan};
use zisk_pil::{MEM_ALIGN_AIR_IDS, ZISK_AIRGROUP_ID};

pub struct MemAlignPlanner<'a> {
    instances: Vec<Plan>,
    num_rows: u32,
    current_skip: u32,
    current_chunk_id: Option<ChunkId>,
    current_chunks: Vec<ChunkId>,
    current_rows_available: u32,
    counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
}

// TODO: dynamic
const MEM_ALIGN_ROWS: usize = 1 << 21;

impl<'a> MemAlignPlanner<'a> {
    pub fn new(counters: Arc<Vec<(ChunkId, &'a MemCounters)>>) -> Self {
        Self {
            instances: Vec::new(),
            num_rows: MEM_ALIGN_ROWS as u32,
            current_skip: 0,
            current_chunk_id: None,
            current_chunks: Vec::new(),
            current_rows_available: MEM_ALIGN_ROWS as u32,
            counters,
        }
    }
    pub fn align_plan(&mut self) -> Vec<Plan> {
        if self.counters.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }

        let count = self.counters.len();
        for index in 0..count {
            let chunk_id = self.counters[index].0;
            let counter = self.counters[index].1;
            self.set_current_chunk_id(chunk_id);
            self.add_to_current_instance(counter.mem_align_rows, &counter.mem_align);
        }
        self.close_current_instance();
        vec![]
    }
    fn set_current_chunk_id(&mut self, chunk_id: ChunkId) {
        if self.current_chunk_id == Some(chunk_id) && !self.current_chunks.is_empty() {
            return;
        }
        self.current_chunk_id = Some(chunk_id);
        if let Err(pos) = self.current_chunks.binary_search(&chunk_id) {
            self.current_chunks.insert(pos, chunk_id);
        }
    }
    fn add_to_current_instance(&mut self, total_rows: u32, operations_rows: &[u8]) {
        let mut pending_rows = total_rows;
        let mut operations_rows_offset: u32 = 0;
        loop {
            // check if has available rows to add all inside this chunks.
            let (count, rows_fit) = if self.current_rows_available >= pending_rows {
                // self.current_rows_available -= pending_rows;
                (0, pending_rows)
            } else {
                self.calculate_how_many_operations_fit(operations_rows_offset, operations_rows)
            };
            self.current_rows_available -= rows_fit;
            pending_rows -= rows_fit;
            if self.current_rows_available == 0 {
                self.close_current_instance();
            }
            operations_rows_offset += count;
            self.open_new_instance(operations_rows_offset, pending_rows > 0);
        }
    }
    fn close_current_instance(&mut self) {
        // TODO: add instance
        if self.current_chunks.is_empty() {
            return;
        }
        // TODO: add multi chunk_id, with skip
        let chunks = std::mem::take(&mut self.current_chunks);
        let instance = Plan::new(
            ZISK_AIRGROUP_ID,
            MEM_ALIGN_AIR_IDS[0],
            Some(self.instances.len()),
            InstanceType::Instance,
            CheckPoint::Multiple(chunks),
            Some(Box::new(CollectInfoSkip::new(self.current_skip as u64))),
            None,
        );
        self.instances.push(instance);
    }
    fn open_new_instance(&mut self, next_instance_skip: u32, use_current_chunk_id: bool) {
        self.current_skip = next_instance_skip;
        self.current_rows_available = self.num_rows;
        if use_current_chunk_id {
            self.current_chunks.push(self.current_chunk_id.unwrap());
        }
    }
    fn calculate_how_many_operations_fit(
        &self,
        operations_offset: u32,
        operations_rows: &[u8],
    ) -> (u32, u32) {
        let mut count = 0;
        let mut rows = 0;
        for row in operations_rows.iter().skip(operations_offset as usize) {
            if (rows + *row as u32) > self.current_rows_available {
                break;
            }
            count += 1;
            rows += *row as u32;
        }
        (count, rows)
    }
}

impl MemPlanCalculator for MemAlignPlanner<'_> {
    fn plan(&mut self) {
        self.align_plan();
    }
    fn collect_plans(&mut self) -> Vec<Plan> {
        std::mem::take(&mut self.instances)
    }
}
