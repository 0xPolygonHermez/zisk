use std::collections::HashMap;

use mem_common::MemAlignCheckPoint;
use zisk_common::{CheckPoint, ChunkId, CollectCounter, InstanceType, Plan, SegmentId};
use zisk_pil::ZISK_AIRGROUP_ID;

const MA_TYPES: usize = 5;

#[derive(Debug, Default, Clone)]
pub struct MemAlignCollectData {
    pub skip: u32,
    pub count: u32,
    // cost in rows, cost == 0 ==> could not consumed
    pub cost: u32,
}

impl MemAlignCollectData {
    pub fn new(cost: u32) -> Self {
        Self { skip: 0, count: 0, cost }
    }
    pub fn add(&mut self, count: u32, skip: u32) -> bool {
        if self.count == 0 {
            self.skip = skip;
            self.count = count;
            true
        } else {
            self.count += count;
            false
        }
    }
}

#[derive(Debug, Default)]
pub struct MemAlignInstanceCounter {
    instances: u32,
    instances_available: u32,
    pub air_id: usize,
    pub num_rows: u32,
    pub rows_available: u32,
    pub chunks: Vec<ChunkId>,
    pub checkpoints: HashMap<ChunkId, MemAlignCheckPoint>,
    pub plans: Vec<Plan>,
    pub collect_data: [MemAlignCollectData; MA_TYPES],
    pub used: [u32; MA_TYPES],
    pub order: Vec<usize>,
}

impl MemAlignInstanceCounter {
    pub fn new(
        air_id: usize,
        instances: u32,
        num_rows: u32,
        costs: &[u32; MA_TYPES],
        order: &[usize],
    ) -> Self {
        Self {
            air_id,
            instances,
            instances_available: instances,
            num_rows,
            rows_available: 0,
            chunks: Vec::new(),
            collect_data: [
                MemAlignCollectData::new(costs[0]),
                MemAlignCollectData::new(costs[1]),
                MemAlignCollectData::new(costs[2]),
                MemAlignCollectData::new(costs[3]),
                MemAlignCollectData::new(costs[4]),
            ],
            order: order.to_vec(),
            checkpoints: HashMap::new(),
            plans: Vec::new(),
            used: [0; MA_TYPES],
        }
    }
    pub fn get_instances(&self) -> u32 {
        self.instances
    }
    pub fn get_instances_available(&self) -> u32 {
        self.instances_available
    }
    pub fn set_instances(&mut self, instances: u32) {
        self.instances = instances;
        self.instances_available = instances;
    }
    pub fn update_order(&mut self, order: &[usize]) {
        self.order = order.to_vec();
    }
    pub fn get_used(&self) -> [u32; MA_TYPES] {
        self.used
    }
    pub fn add_to_instance(
        &mut self,
        chunk_id: ChunkId,
        totals: &[u32; MA_TYPES],
        pendings: &mut [u32; MA_TYPES],
    ) {
        let mut updated = false;
        let count = self.order.len();
        for j in 0..count {
            let i = self.order[j];
            let cost = self.collect_data[i].cost;
            if cost == 0 {
                continue;
            }
            while pendings[i] > 0 {
                let total = totals[i];
                let pending = pendings[i];
                let cost_pending = cost * pending;
                if self.rows_available < cost {
                    // before open a new instance, need to close chunk if there are data.
                    if updated {
                        // for this segment this chunk was closed
                        self.close_chunk(chunk_id);
                        updated = false;
                    }
                    if !self.close_and_open_instance() {
                        // no more instances free
                        break;
                    }
                }

                if cost_pending <= self.rows_available {
                    // could add all pending
                    self.collect_data[i].add(pending, total - pending);
                    self.used[i] += pending;
                    self.rows_available -= cost_pending;
                    pendings[i] = 0;
                    updated = true;
                    break;
                }

                let partial = self.rows_available / cost;

                // partial = 0 ==> self.rows_available < cost, but
                // this condition was evaluated before open a new instance, only two cases:
                // - open new instances => self.rows_available > cost
                // - no more instances => exit with break;
                assert!(partial > 0);

                self.collect_data[i].add(partial, total - pendings[i]);
                self.used[i] += partial;
                self.rows_available -= partial * cost;
                pendings[i] -= partial;
                updated = true;
            }
        }
        if updated {
            self.close_chunk(chunk_id);
        }
    }
    fn clear_collect_data(&mut self) {
        for i in 0..MA_TYPES {
            self.collect_data[i].skip = 0;
            self.collect_data[i].count = 0;
        }
    }
    fn close_chunk(&mut self, chunk_id: ChunkId) {
        let checkpoint = MemAlignCheckPoint {
            air_id: self.air_id,
            chunk_id,
            full_5: CollectCounter::new(self.collect_data[0].skip, self.collect_data[0].count),
            full_3: CollectCounter::new(self.collect_data[1].skip, self.collect_data[1].count),
            full_2: CollectCounter::new(self.collect_data[2].skip, self.collect_data[2].count),
            read_byte: CollectCounter::new(self.collect_data[3].skip, self.collect_data[3].count),
            write_byte: CollectCounter::new(self.collect_data[4].skip, self.collect_data[4].count),
        };
        self.clear_collect_data();
        self.chunks.push(chunk_id);
        assert!(self.checkpoints.insert(chunk_id, checkpoint).is_none());
    }
    fn close_and_open_instance(&mut self) -> bool {
        self.close_instance();
        self.open_new_instance()
    }
    fn open_new_instance(&mut self) -> bool {
        if self.instances_available == 0 {
            false
        } else {
            self.rows_available = self.num_rows;
            self.instances_available -= 1;
            self.chunks.clear();
            true
        }
    }
    #[cfg(feature = "mem_align_stats")]
    pub fn get_total_counts(&self) -> (u32, u32, u32, u32, u32) {
        let mut full_5 = 0;
        let mut full_3 = 0;
        let mut full_2 = 0;
        let mut read_byte = 0;
        let mut write_byte = 0;
        for checkpoint in self.checkpoints.values() {
            full_5 += checkpoint.full_5.count();
            full_3 += checkpoint.full_3.count();
            full_2 += checkpoint.full_2.count();
            read_byte += checkpoint.read_byte.count();
            write_byte += checkpoint.write_byte.count();
        }
        (full_5, full_3, full_2, read_byte, write_byte)
    }

    pub fn close_instance(&mut self) {
        if self.rows_available == self.num_rows || self.chunks.is_empty() {
            return;
        }
        let segment_id = SegmentId(self.plans.len());
        #[cfg(feature = "mem_align_stats")]
        {
            use zisk_pil::MEM_ALIGN_AIR_IDS;

            let totals = self.get_total_counts();
            let total = if self.air_id == MEM_ALIGN_AIR_IDS[0] {
                5 * totals.0 + 3 * totals.1 + 2 * totals.2 + 2 * totals.3 + 3 * totals.4
            } else {
                totals.0 + totals.1 + totals.2 + totals.3 + totals.4
            };
            println!("MEM_ALIGN_SEGMENT AIR:{} SEGMENT:{} FULL_5:{} FULL_3:{} FULL_2:{} READ_BYTE:{} WRITE_BYTE:{} TOTAL:{total}",
                self.air_id, segment_id, totals.0, totals.1, totals.2, totals.3, totals.4);
        }
        let chunks = std::mem::take(&mut self.chunks);
        let checkpoints = std::mem::take(&mut self.checkpoints);
        let plan = Plan::new(
            ZISK_AIRGROUP_ID,
            self.air_id,
            Some(segment_id),
            InstanceType::Instance,
            CheckPoint::Multiple(chunks),
            Some(Box::new(checkpoints)),
            4,
        );
        self.plans.push(plan);
    }
}
