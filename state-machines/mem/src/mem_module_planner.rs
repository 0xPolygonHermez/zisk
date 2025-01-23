use std::sync::Arc;

use crate::{MemCounters, MemHelpers, MemPlanCalculator, UsesCounter, STEP_MEMORY_MAX_DIFF};
use sm_common::{CheckPoint, ChunkId, InstanceType, Plan};

const REGISTERS_COUNT: usize = 32;

#[derive(Debug)]
struct MemModuleSegment {
    pub prev_addr: u32,
    pub last_addr: u32,
    pub skip_rows: u32,
    pub rows: u32,
    pub prev_step: u64,
    pub last_step: u64,
    pub prev_value: u64,
    pub chunks: Vec<ChunkId>,
}
pub struct MemModulePlanner<'a> {
    config: MemModulePlannerConfig,
    rows_available: u32,
    last_step: u64,
    last_addr: u32,  // addr of last addr uses
    last_value: u64, // value of last addr uses
    cursors: Vec<(usize, usize)>,
    segments: Vec<MemModuleSegment>,
    current_chunk_id: Option<ChunkId>,
    counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
    consume_addr: u32,
    consume_from_step: u64,
    consume_to_step: u64,
}

#[derive(Debug, Default, Clone)]
pub struct MemModuleSegmentCheckPoint {
    pub prev_addr: u32,
    pub last_addr: u32,
    pub skip_rows: u32,
    pub rows: u32,
    pub is_last_segment: bool,
    pub prev_step: u64,
    pub last_step: u64,
    pub prev_value: u64,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct MemModulePlannerConfig {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub from_addr: u32,
    pub to_addr: u32,
    pub rows: u32,
    pub map_registers: bool,
    pub consecutive_addr: bool,
    pub intermediate_step_reads: bool,
}
impl<'a> MemModulePlanner<'a> {
    pub fn new(
        config: MemModulePlannerConfig,
        counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
    ) -> Self {
        Self {
            config,
            last_addr: config.from_addr,
            last_step: 0,
            last_value: 0,
            rows_available: config.rows,
            cursors: Vec::new(),
            segments: Vec::new(),
            counters,
            current_chunk_id: None,
            consume_addr: 0,
            consume_from_step: 0,
            consume_to_step: 0,
        }
    }
    pub fn module_plan(&mut self) {
        if self.counters.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }
        self.open_initial_segment();
        // create a list of cursors, this list has the non-empty indexs of metric (couters) and his
        // cursor init to first position
        self.init_cursors();
        while !self.cursors.is_empty() {
            // searches for the first smallest element in the vector and returns its index.
            let (cursor_index, cursor_pos) = self.get_next_cursor_index_and_pos();

            let chunk_id = self.get_cursor_chunk_id(cursor_index);
            let (addr, addr_uses) = self.get_cursor_data(cursor_index, cursor_pos);
            self.add_to_current_instance(chunk_id, addr, &addr_uses);
        }
    }
    fn get_cursor_chunk_id(&self, cursor_index: usize) -> ChunkId {
        self.counters[cursor_index].0
    }
    fn get_cursor_data(&self, cursor_index: usize, cursor_pos: usize) -> (u32, UsesCounter) {
        let result = if cursor_pos < REGISTERS_COUNT {
            (
                MemHelpers::register_to_addr_w(cursor_pos as u8),
                self.counters[cursor_index].1.registers[cursor_pos],
            )
        } else {
            let addr = self.counters[cursor_index].1.addr_sorted[cursor_pos - REGISTERS_COUNT].0;
            let addr_uses =
                self.counters[cursor_index].1.addr_sorted[cursor_pos - REGISTERS_COUNT].1;
            (addr, addr_uses)
        };
        debug_assert!(
            result.0 >= self.config.from_addr && result.0 <= self.config.to_addr,
            "INVALID_CURSOR addr:{:#010X} from_addr:{:#010X} to_addr:{:#010X} cursor:[{},{}]",
            result.0 * 8,
            self.config.from_addr * 8,
            self.config.to_addr * 8,
            cursor_index,
            cursor_pos,
        );
        result
    }
    fn init_cursors(&mut self) {
        // for each chunk-counter that has addr_sorted element add a cursor to the first element
        self.cursors = Vec::new();
        for (index, counter) in self.counters.iter().enumerate() {
            let mut jmp_to_next = false;
            // first take registers if them are mapped on top of the current memory area
            if self.config.map_registers {
                for ireg in 0..counter.1.registers.len() {
                    if counter.1.registers[ireg].count > 0 {
                        self.cursors.push((index, ireg));
                        jmp_to_next = true;
                        break;
                    }
                }
            }
            if jmp_to_next || counter.1.addr_sorted.is_empty() {
                continue;
            }
            match counter.1.addr_sorted.binary_search_by(|(key, _)| key.cmp(&self.config.from_addr))
            {
                Ok(pos) => self.cursors.push((index, pos + REGISTERS_COUNT)),
                Err(pos) => {
                    if pos < counter.1.addr_sorted.len() &&
                        counter.1.addr_sorted[pos].0 <= self.config.to_addr
                    {
                        self.cursors.push((index, pos + REGISTERS_COUNT));
                    }
                }
            }
        }

        #[cfg(debug_assertions)]
        for (cursor_index, cursor_pos) in self.cursors.iter() {
            let (_, _) = self.get_cursor_data(*cursor_index, *cursor_pos);
        }
    }
    fn get_next_cursor_index_and_pos(&mut self) -> (usize, usize) {
        let (min_index, _) = self
            .cursors
            .iter()
            .enumerate()
            .min_by_key(|&(_, &(index, cursor))| {
                if cursor < REGISTERS_COUNT {
                    MemHelpers::register_to_addr_w(cursor as u8)
                } else {
                    self.counters[index].1.addr_sorted[cursor - REGISTERS_COUNT].0
                }
            })
            .unwrap();
        let cursor_index = self.cursors[min_index].0;
        let cursor_pos = self.cursors[min_index].1;

        let cursor_on_registers = cursor_pos < REGISTERS_COUNT;
        let mut cursor_next_pos = if cursor_on_registers {
            for ireg in cursor_pos + 1..REGISTERS_COUNT {
                if self.counters[cursor_index].1.registers[ireg].count > 0 {
                    self.cursors[min_index].1 = ireg;
                    return (cursor_index, cursor_pos);
                }
            }
            REGISTERS_COUNT
        } else {
            cursor_pos + 1
        };

        let mut cursor_next_addr_pos = cursor_next_pos - REGISTERS_COUNT;
        if cursor_on_registers &&
            cursor_next_addr_pos < self.counters[cursor_index].1.addr_sorted.len()
        {
            // filter addr out of [from_addr, to_addr]
            while cursor_next_addr_pos < self.counters[cursor_index].1.addr_sorted.len() &&
                self.counters[cursor_index].1.addr_sorted[cursor_next_addr_pos].0 <
                    self.config.from_addr
            {
                cursor_next_addr_pos += 1;
                cursor_next_pos += 1;
            }
        }

        // if it's last position, we must remove for list of open_cursors, if not we increment
        if cursor_next_addr_pos >= self.counters[cursor_index].1.addr_sorted.len() ||
            self.counters[cursor_index].1.addr_sorted[cursor_next_addr_pos].0 >
                self.config.to_addr
        {
            self.cursors.remove(min_index);
        } else {
            self.cursors[min_index].1 = cursor_next_pos;
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
        self.add_block_of_addr_uses(addr, addr_uses);
    }
    fn add_block_of_addr_uses(&mut self, addr: u32, addr_uses: &UsesCounter) {
        let mut skip_rows = 0;
        let mut pending_rows = addr_uses.count;
        self.set_consume_info(addr, addr_uses.first_step, addr_uses.last_step);
        while pending_rows > 0 {
            match (self.rows_available as u64).cmp(&pending_rows) {
                std::cmp::Ordering::Greater => {
                    self.consume_rows(pending_rows as u32, 1);
                    break;
                }
                std::cmp::Ordering::Equal => {
                    self.consume_rows(pending_rows as u32, 2);
                    self.close_and_open_segment(
                        addr,
                        // if the block fit inside current segment, the last step is last_step,
                        // means the previous step of last segment was last_step
                        addr_uses.last_step,
                        addr_uses.last_value,
                    );
                    break;
                }
                std::cmp::Ordering::Less => {
                    let rows_applied = self.rows_available;
                    self.consume_rows(rows_applied, 3);
                    pending_rows -= rows_applied as u64;
                    skip_rows += rows_applied;
                    self.close_and_open_segment_with_skip(
                        addr,
                        addr_uses.first_step,
                        addr,
                        addr_uses.last_step,
                        0,
                        // when we skipping inputs, the previous addr/step is naturally discarted
                        // because it belongs to the previous segment, for this reason we skip 1
                        // row
                        // rows_applied = 0 => never, because means no more space in current
                        // segment but always open a new segment after close a segment.
                        skip_rows - 1,
                    );
                }
            }
        }
        self.last_value = addr_uses.last_value;
        self.last_step = addr_uses.last_step;
        self.last_addr = addr;
        self.update_segment();
    }

    fn set_current_chunk_id(&mut self, chunk_id: ChunkId) {
        let lindex = self.segments.len() - 1;
        if self.current_chunk_id == Some(chunk_id) && !self.segments[lindex].chunks.is_empty() {
            return;
        }
        self.current_chunk_id = Some(chunk_id);
        self.insert_current_chunk_id_to_segment();
    }
    fn insert_current_chunk_id_to_segment(&mut self) {
        let lindex = self.segments.len() - 1;
        if let Some(chunk_id) = self.current_chunk_id {
            if let Err(pos) = self.segments[lindex].chunks.binary_search(&chunk_id) {
                self.segments[lindex].chunks.insert(pos, chunk_id);
            }
        }
    }
    fn set_consume_info(&mut self, addr: u32, from_step: u64, to_step: u64) {
        self.consume_addr = addr;
        self.consume_from_step = from_step;
        self.consume_to_step = to_step;
    }
    fn consume_rows(&mut self, rows: u32, _label: u32) {
        let lindex = self.segments.len() - 1;

        if self.segments[lindex].rows == 0 {
            self.insert_current_chunk_id_to_segment();
        }

        self.segments[lindex].rows += rows;
        self.rows_available -= rows;
        debug_assert!(
            self.rows_available + self.segments[lindex].rows == self.config.rows,
            "rows_available:{} rows:{} config.rows:{}",
            self.rows_available,
            self.segments[lindex].rows,
            self.config.rows
        );
    }
    fn add_internal_rows(&mut self, addr: u32, count: u32, addr_inc: u32, step_inc: u32) {
        // check if all internal reads fit in the current instance
        let mut pending = count;
        self.set_consume_info(addr, self.last_step, self.last_step + (step_inc * count) as u64);
        loop {
            // if pending <= self.rows_available {
            if pending < self.rows_available {
                self.consume_rows(pending, 4);
                pending = 0;
            } else {
                let rows_applied = self.rows_available;
                self.consume_rows(rows_applied, 5);
                pending -= rows_applied;
                let segment_last_addr = addr + (count - pending) * addr_inc;
                let segment_last_step = self.last_step + step_inc as u64 * (count - pending) as u64;
                // with informatio we don't need to skip anything, we skip only when we don't
                // have information (addr, step) of previous instance
                self.close_and_open_segment(segment_last_addr, segment_last_step, self.last_value);
            }
            if pending == 0 {
                break;
            }
        }
        self.last_addr = addr + count * addr_inc;
        self.last_step += step_inc as u64 * count as u64;
        self.update_segment();
    }
    fn add_internal_reads_to_current_instance(&mut self, addr: u32, addr_uses: &UsesCounter) {
        if self.last_addr != addr {
            if self.config.consecutive_addr && addr - self.last_addr > 1 {
                // adding internal reads of zero for consecutive addresses
                self.add_internal_rows(self.last_addr + 1, addr - self.last_addr - 1, 1, 0);
            }
            return;
        }

        if !self.config.intermediate_step_reads {
            return;
        }

        let step_diff = addr_uses.first_step - self.last_step;
        if step_diff <= STEP_MEMORY_MAX_DIFF {
            return;
        }

        // at this point we need to add internal reads, we calculate how many internal reads we need
        let internal_rows = (step_diff - 1) / STEP_MEMORY_MAX_DIFF;
        assert!(internal_rows < self.config.rows as u64);
        self.add_internal_rows(addr, internal_rows as u32, 0, STEP_MEMORY_MAX_DIFF as u32);
    }
    fn open_initial_segment(&mut self) {
        self.segments.push({
            MemModuleSegment {
                prev_addr: self.config.from_addr,
                prev_step: 0,
                prev_value: 0,
                last_addr: 0,
                last_step: 0,
                skip_rows: 0,
                rows: 0,
                chunks: Vec::new(),
            }
        });
    }
    fn update_segment(&mut self) {
        let lindex = self.segments.len() - 1;
        self.segments[lindex].last_addr = self.last_addr;
        self.segments[lindex].last_step = self.last_step;
    }
    fn close_segment(&mut self, last_addr: u32, last_step: u64) {
        let lindex = self.segments.len() - 1;
        self.segments[lindex].last_addr = last_addr;
        self.segments[lindex].last_step = last_step;
        self.rows_available = self.config.rows;
    }
    fn close_and_open_segment(&mut self, last_addr: u32, last_step: u64, last_value: u64) {
        self.close_segment(last_addr, last_step);

        self.segments.push({
            MemModuleSegment {
                prev_addr: last_addr,
                prev_step: last_step,
                prev_value: last_value,
                last_addr,
                last_step,
                skip_rows: 0,
                rows: 0,
                chunks: Vec::new(),
            }
        });
    }
    fn close_and_open_segment_with_skip(
        &mut self,
        prev_addr: u32,
        prev_step: u64,
        last_addr: u32,
        last_step: u64,
        prev_value: u64,
        skip_rows: u32,
    ) {
        self.close_segment(last_addr, last_step);

        self.segments.push({
            MemModuleSegment {
                prev_addr,
                prev_step,
                prev_value,
                last_addr,
                last_step,
                skip_rows,
                rows: 0,
                chunks: Vec::new(),
            }
        });
    }
}

impl MemPlanCalculator for MemModulePlanner<'_> {
    fn plan(&mut self) {
        self.module_plan();
    }
    fn collect_plans(&mut self) -> Vec<Plan> {
        let mut plans: Vec<Plan> = Vec::new();
        if self.segments.is_empty() || self.segments.last().unwrap().rows == 0 {
            // no data => no plans
            return plans;
        }

        let last_segment_id = if self.segments.last().unwrap().rows > 0 {
            self.segments.len() - 1
        } else {
            self.segments.len() - 2
        };
        for (segment_id, segment) in self.segments.iter().enumerate() {
            let check_point = MemModuleSegmentCheckPoint {
                prev_addr: segment.prev_addr,
                last_addr: segment.last_addr,
                skip_rows: segment.skip_rows,
                rows: segment.rows,
                is_last_segment: segment_id == last_segment_id,
                prev_step: segment.prev_step,
                last_step: segment.last_step,
                prev_value: segment.prev_value,
            };

            plans.push(Plan::new(
                self.config.airgroup_id,
                self.config.air_id,
                Some(segment_id),
                InstanceType::Instance,
                CheckPoint::Multiple(segment.chunks.clone()),
                None,
                Some(Box::new(check_point)),
            ));

            // exit in this point to prevent an last empty segment
            if segment_id == last_segment_id {
                break;
            }
        }
        plans
    }
}
