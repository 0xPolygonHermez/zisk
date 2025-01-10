use std::sync::Arc;

use crate::{MemCounters, MemHelpers, MemPlanCalculator, UsesCounter, MEMORY_MAX_DIFF};
use log::info;
use sm_common::{CheckPoint, ChunkId, InstanceType, Plan};

pub struct MemModulePlanner<'a> {
    airgroup_id: usize,
    air_id: usize,
    from_addr: u32,
    to_addr: u32,
    rows_available: u32,
    instance_rows: u32,
    last_step: u64,
    last_addr: u32,  // addr of last addr uses
    last_value: u64, // value of last addr uses
    cursors: Vec<(usize, usize)>,
    pub instances: Vec<Plan>,
    first_instance: bool,
    current_checkpoint_chunks: Vec<ChunkId>,
    current_checkpoint: MemInstanceCheckPoint,
    current_chunk_id: Option<ChunkId>,
    counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
}

#[derive(Debug, Default, Clone)]
pub struct MemInstanceCheckPoint {
    pub prev_addr: u32,
    pub last_addr: u32,
    pub skip_internal: u32,
    pub is_last_segment: bool,
    pub prev_step: u64,
    pub last_step: u64,
    pub prev_value: u64,
}

impl<'a> MemModulePlanner<'a> {
    pub fn new(
        airgroup_id: usize,
        air_id: usize,
        from_addr: u32,
        to_addr: u32,
        counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
    ) -> Self {
        Self {
            airgroup_id,
            air_id,
            from_addr,
            to_addr,
            last_addr: 0,
            last_step: 0,
            last_value: 0,
            rows_available: 0,
            instance_rows: 1 << 21,
            cursors: Vec::new(),
            instances: Vec::new(),
            counters,
            first_instance: true,
            current_chunk_id: None,
            current_checkpoint_chunks: Vec::new(),
            current_checkpoint: MemInstanceCheckPoint {
                prev_addr: from_addr,
                last_addr: 0,
                last_step: 0,
                skip_internal: 0,
                prev_step: 0,
                prev_value: 0,
                is_last_segment: false,
            },
        }
    }
    pub fn module_plan(&mut self) {
        if self.counters.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }
        info!("[Mem]   MemModulePlan {}", self.counters.len());

        // create a list of cursors, this list has the non-empty indexs of metric (couters) and his
        // cursor init to first position
        self.init_cursors();

        while !self.cursors.is_empty() {
            // searches for the first smallest element in the vector and returns its index.
            let (cursor_index, cursor_pos) = self.get_next_cursor_index_and_pos();

            let chunk_id = self.counters[cursor_index].0;
            let addr = self.counters[cursor_index].1.addr_sorted[cursor_pos].0;
            let addr_uses = self.counters[cursor_index].1.addr_sorted[cursor_pos].1;

            self.add_to_current_instance(chunk_id, addr, &addr_uses);
        }
    }
    fn init_cursors(&mut self) {
        // for each chunk-counter that has addr_sorted element add a cursor to the first element
        self.cursors = Vec::new();
        for (index, counter) in self.counters.iter().enumerate() {
            if counter.1.addr_sorted.is_empty() {
                continue;
            }
            match counter.1.addr_sorted.binary_search_by(|(key, _)| key.cmp(&self.from_addr)) {
                Ok(pos) => self.cursors.push((index, pos)),
                Err(pos) => {
                    if pos < counter.1.addr_sorted.len() &&
                        counter.1.addr_sorted[pos].0 <= self.to_addr
                    {
                        self.cursors.push((index, pos));
                    }
                }
            }
        }
    }
    fn get_next_cursor_index_and_pos(&mut self) -> (usize, usize) {
        let (min_index, _) = self
            .cursors
            .iter()
            .enumerate()
            .min_by_key(|&(_, &(index, cursor))| self.counters[index].1.addr_sorted[cursor].0)
            .unwrap();
        let cursor_index = self.cursors[min_index].0;
        let cursor_pos = self.cursors[min_index].1;

        // if it's last position, we must remove for list of open_cursors, if not we increment
        if cursor_pos + 1 >= self.counters[cursor_index].1.addr_sorted.len() ||
            self.counters[cursor_index].1.addr_sorted[cursor_pos + 1].0 > self.to_addr
        {
            self.cursors.remove(min_index);
        } else {
            self.cursors[min_index].1 += 1;
        }
        (cursor_index, cursor_pos)
    }
    /// Add "counter-address" to the current instance
    /// If the chunk_id is not in the list, it will be added. This method need to verify the
    /// distance between the last addr-step and the current addr-step, if the distance is
    /// greater than MEMORY_MAX_DIFF we need to add extra intermediate "steps".
    fn add_to_current_instance(&mut self, chunk_id: ChunkId, addr: u32, addr_uses: &UsesCounter) {
        self.set_current_chunk_id(chunk_id);
        self.add_internal_reads_to_current_instance(addr, addr_uses);

        let mut pending_rows = addr_uses.count;
        while pending_rows > 0 {
            if self.rows_available as u64 > pending_rows {
                self.rows_available -= pending_rows as u32;
                break;
            }
            pending_rows -= self.rows_available as u64;
            let skip_internal = self.rows_available;
            self.rows_available = 0;
            self.close_instance();
            self.open_instance(
                addr,
                0, // unknown this intermediate value
                0, // unknown this intermediate step
                skip_internal,
            );

            self.rows_available = self.instance_rows;
        }
        // update last_xxx
        self.last_value = addr_uses.last_value;
        self.last_step = addr_uses.last_step;
        self.last_addr = addr;
    }
    fn close_instance(&mut self) {
        if self.current_checkpoint_chunks.is_empty() {
            return;
        }
        // TODO: add chunks
        // for chunk_id in self.current_checkpoint_chunks.iter() {
        //     instance.add_chunk_id(chunk_id.clone());
        // }

        let mut checkpoint = std::mem::take(&mut self.current_checkpoint);
        checkpoint.last_addr = self.last_addr;
        checkpoint.last_step = self.last_step;

        let chunks = std::mem::take(&mut self.current_checkpoint_chunks);
        let instance = Plan::new(
            self.airgroup_id,
            self.air_id,
            Some(self.instances.len()),
            InstanceType::Instance,
            CheckPoint::Multiple(chunks),
            None,
            Some(Box::new(checkpoint)),
        );
        self.instances.push(instance);
    }
    fn set_current_chunk_id(&mut self, chunk_id: ChunkId) {
        if self.current_chunk_id == Some(chunk_id) && !self.current_checkpoint_chunks.is_empty() {
            return;
        }
        self.current_chunk_id = Some(chunk_id);
        if let Err(pos) = self.current_checkpoint_chunks.binary_search(&chunk_id) {
            self.current_checkpoint_chunks.insert(pos, chunk_id);
        }
    }
    fn add_internal_reads_to_current_instance(&mut self, addr: u32, addr_uses: &UsesCounter) {
        // check internal reads (update last_xxx)
        // reopen instance if need and set his chunk_id
        if self.last_addr != addr {
            return;
        }

        // TODO: dynamic by addr mapping
        if !MemHelpers::step_extra_reads_enabled(addr) {
            return;
        }

        let step_diff = addr_uses.first_step - self.last_step;
        if step_diff <= MEMORY_MAX_DIFF {
            return;
        }

        // at this point we need to add internal reads, we calculate how many internal reads we need
        let mut internal_rows = (step_diff - 1) / MEMORY_MAX_DIFF;
        assert!(internal_rows < self.instance_rows as u64);

        // check if all internal reads fit in the current instance
        if internal_rows < self.rows_available as u64 {
            self.rows_available -= internal_rows as u32;
        } else {
            internal_rows -= self.rows_available as u64;
            let skip_internal = self.rows_available;
            self.rows_available = 0;
            self.close_instance();
            self.open_instance(
                addr,
                self.last_value,
                self.last_step + MEMORY_MAX_DIFF * skip_internal as u64,
                skip_internal,
            );

            // rows_available is the number of rows after substract "pending" internal rows
            self.rows_available = self.instance_rows - internal_rows as u32;
        }
    }
    fn open_instance(
        &mut self,
        prev_addr: u32,
        prev_value: u64,
        prev_step: u64,
        skip_internal: u32,
    ) {
        // TODO: add current chunk_id to new instance
        self.first_instance = false;
        self.current_checkpoint.prev_addr = prev_addr;
        self.current_checkpoint.skip_internal = skip_internal;
        self.current_checkpoint.prev_step = prev_step;

        // TODO: IMPORTANT review, when change of instance we need to known the previous value on
        // write (on read previous value and current must be the same)
        self.current_checkpoint.prev_value = prev_value;

        // TODO: add current chunk id
    }
}

impl MemPlanCalculator for MemModulePlanner<'_> {
    fn plan(&mut self) {
        self.module_plan();
    }
    fn collect_plans(&mut self) -> Vec<Plan> {
        std::mem::take(&mut self.instances)
    }
}
