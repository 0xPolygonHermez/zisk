use std::{collections::HashMap, sync::Arc};

use crate::{MemCounters, MemPlanCalculator};
use mem_common::MemAlignCheckPoint;
use proofman_common::PreCalculate;
use zisk_common::{CheckPoint, ChunkId, InstanceType, Plan, SegmentId};
use zisk_pil::{MemAlignTrace, MEM_ALIGN_AIR_IDS, MEM_ALIGN_ROM_AIR_IDS, ZISK_AIRGROUP_ID};
#[allow(dead_code)]
pub struct MemAlignPlanner<'a> {
    instances: Vec<Plan>,
    num_rows: u32,
    chunk_id: Option<ChunkId>,
    chunks: Vec<ChunkId>,
    check_points: HashMap<ChunkId, MemAlignCheckPoint>,
    rows_available: u32,
    counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
}

impl<'a> MemAlignPlanner<'a> {
    pub fn new(counters: Arc<Vec<(ChunkId, &'a MemCounters)>>) -> Self {
        let num_rows = MemAlignTrace::<usize>::NUM_ROWS as u32;
        Self {
            instances: Vec::new(),
            num_rows,
            chunk_id: None,
            chunks: Vec::new(),
            rows_available: num_rows,
            check_points: HashMap::new(),
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
            if counter.mem_align_rows == 0 {
                continue;
            };
            self.set_current_chunk_id(chunk_id);
            self.add_to_current_instance(counter.mem_align_rows, &counter.mem_align);
        }
        self.close_current_instance();
        vec![]
    }
    fn set_current_chunk_id(&mut self, chunk_id: ChunkId) {
        if self.chunk_id == Some(chunk_id) && !self.chunks.is_empty() {
            return;
        }
        self.chunk_id = Some(chunk_id);
        if let Err(pos) = self.chunks.binary_search(&chunk_id) {
            self.chunks.insert(pos, chunk_id);
        }
    }
    fn add_to_current_instance(&mut self, total_rows: u32, operations_rows: &[u8]) {
        let mut pending_rows = total_rows;
        // operations_rows is array of number of rows per operation, operations_done is the
        // number of operations processed, for each new instance is the number of operations to skip
        let mut operations_done: u32 = 0;
        loop {
            // check if has available rows to add all inside this chunks.
            let (count, rows_fit) = if self.rows_available >= pending_rows {
                // remaing operations is number of operations - operations_done
                (operations_rows.len() as u32 - operations_done, pending_rows)
            } else {
                self.calculate_how_many_operations_fit(operations_done, operations_rows)
            };

            if count != 0 {
                self.check_points.insert(
                    self.chunk_id.unwrap(),
                    MemAlignCheckPoint {
                        skip: operations_done,
                        count,
                        rows: rows_fit,
                        offset: self.num_rows - self.rows_available,
                    },
                );
            }

            self.rows_available -= rows_fit;
            pending_rows -= rows_fit;
            operations_done += count;

            if self.rows_available == 0 || rows_fit == 0 {
                let use_current_chunk_id = pending_rows > 0;
                self.close_current_instance();
                self.open_new_instance(use_current_chunk_id);
            }
            if pending_rows == 0 {
                break;
            }
        }
    }
    fn close_current_instance(&mut self) {
        // TODO: add instance
        if self.chunks.is_empty() {
            return;
        }
        // TODO: add multi chunk_id, with skip
        let chunks = std::mem::take(&mut self.chunks);
        let check_points = std::mem::take(&mut self.check_points);

        let instance = Plan::new(
            ZISK_AIRGROUP_ID,
            MEM_ALIGN_AIR_IDS[0],
            Some(SegmentId(self.instances.len())),
            InstanceType::Instance,
            CheckPoint::Multiple(chunks),
            PreCalculate::Fast,
            Some(Box::new(check_points)),
        );
        self.instances.push(instance);
    }
    fn open_new_instance(&mut self, use_current_chunk_id: bool) {
        self.rows_available = self.num_rows;
        if use_current_chunk_id {
            self.chunks.push(self.chunk_id.unwrap());
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
            if (rows + *row as u32) > self.rows_available {
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
        if !self.instances.is_empty() {
            self.instances.push(Plan::new(
                ZISK_AIRGROUP_ID,
                MEM_ALIGN_ROM_AIR_IDS[0],
                None,
                InstanceType::Table,
                CheckPoint::None,
                PreCalculate::None,
                None,
            ));
        }
        std::mem::take(&mut self.instances)
    }
}
