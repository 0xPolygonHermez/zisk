use rayon::{prelude::*, ThreadPoolBuilder};
use std::sync::Arc;

use crate::{MemCounters, MemPlanCalculator, UsesCounter, STEP_MEMORY_MAX_DIFF};
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

    segments: Vec<MemModuleSegment>,
    current_chunk_id: Option<ChunkId>,
    counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
    consume_addr: u32,
    consume_from_step: u64,
    consume_to_step: u64,
    cursor_index: usize,
    cursor_count: usize,
    sorted_boxes: Vec<SortedBox>,
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

#[derive(Debug, Default, Clone)]
pub struct SortedBox {
    pub addr: u64,
    pub i_counter: u32,
    pub i_addr: u32,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct MemModulePlannerConfig {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub addr_index: usize,
    pub from_addr: u32,
    pub to_addr: u32,
    pub rows: u32,
    pub consecutive_addr: bool,
    pub intermediate_step_reads: bool,
}
impl<'a> MemModulePlanner<'a> {
    pub fn new(
        config: MemModulePlannerConfig,
        counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
    ) -> Self {
        let counters_count = counters.len();
        Self {
            config,
            last_addr: config.from_addr,
            last_step: 0,
            last_value: 0,
            rows_available: config.rows,
            segments: Vec::new(),
            counters,
            current_chunk_id: None,
            consume_addr: 0,
            consume_from_step: 0,
            consume_to_step: 0,
            cursor_index: 0,
            cursor_count: counters_count * REGISTERS_COUNT,
            sorted_boxes: Vec::new(),
        }
    }
    pub fn module_plan(&mut self) {
        if self.counters.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }
        self.open_initial_segment();
        // create a list of cursors, this list has the non-empty indexs of metric (couters) and his
        self.init_cursor();
        while !self.cursors_end() {
            // searches for the first smallest element in the vector
            let (chunk_id, addr, addr_uses) = self.get_next_cursor();
            // println!("chunk_id:{:?} addr:{:#010X} addr_uses:{:?}", chunk_id, addr * 8,
            // addr_uses);
            self.add_to_current_instance(chunk_id, addr, &addr_uses);
        }
    }
    fn init_cursor(&mut self) {
        // check if any register has data
        let initial_sorted_boxes = self.prepare_sorted_boxes();
        self.sorted_boxes = self.merge_sorted_boxes(&initial_sorted_boxes, 4);
        self.init_sorted_boxes_cursor();
        #[cfg(feature = "debug_mem")]
        self.debug_sorted_boxes();
    }
    fn cursors_end(&self) -> bool {
        self.cursor_index >= self.cursor_count
    }
    fn get_next_cursor(&mut self) -> (ChunkId, u32, UsesCounter) {
        let aid = self.config.addr_index;
        let counter_index = self.sorted_boxes[self.cursor_index].i_counter as usize;
        let addr_index = self.sorted_boxes[self.cursor_index].i_addr as usize;
        self.cursor_index += 1;
        (
            self.counters[counter_index].0,
            self.counters[counter_index].1.addr_sorted[aid][addr_index].0,
            self.counters[counter_index].1.addr_sorted[aid][addr_index].1,
        )
    }

    #[cfg(feature = "debug_mem")]
    fn debug_sorted_boxes(&self) {
        let aid = self.config.addr_index;
        let mut prev_addr = 0;
        let mut prev_step = 0;
        for (i, box_ref) in self.sorted_boxes.iter().enumerate() {
            let addr = box_ref.addr;
            let _addr = self.counters[box_ref.i_counter as usize].1.addr_sorted[aid]
                [box_ref.i_addr as usize]
                .0;
            let step = self.counters[box_ref.i_counter as usize].1.addr_sorted[aid]
                [box_ref.i_addr as usize]
                .1
                .first_step;
            let order_ok = prev_addr < addr || (prev_addr == addr && prev_step < step);
            println!(
                "#{} addr:{:#10X}({:#10X}){} step:{}{}",
                i,
                addr * 8,
                _addr * 8,
                if addr == _addr as u64 { "" } else { "!!!" },
                step,
                if order_ok { "" } else { " order fail !!!" }
            );
            prev_addr = addr;
            prev_step = step;
        }
    }
    fn init_sorted_boxes_cursor(&mut self) {
        self.cursor_index = 0;
        self.cursor_count = self.sorted_boxes.len();
        // println!("INIT SORTED BOXES CURSOR {}/{}", self.cursor_index, self.cursor_count);
    }

    fn prepare_sorted_boxes(&self) -> Vec<Vec<SortedBox>> {
        let pool = ThreadPoolBuilder::new().num_threads(4).build().unwrap();
        pool.install(|| {
            self.counters
                .par_iter()
                .enumerate()
                .map(|(i, counter)| {
                    let addr_count = counter.1.addr_sorted[self.config.addr_index].len();
                    let mut counter_boxes: Vec<SortedBox> = Vec::with_capacity(addr_count);
                    for i_addr in 0..addr_count {
                        let addr = counter.1.addr_sorted[self.config.addr_index][i_addr].0 as u64;
                        counter_boxes.push(SortedBox {
                            addr,
                            i_counter: i as u32,
                            i_addr: i_addr as u32,
                        });
                    }
                    counter_boxes
                })
                .collect()
        })
    }
    fn merge_sorted_boxes(&self, sorted_boxes: &[Vec<SortedBox>], arity: usize) -> Vec<SortedBox> {
        if sorted_boxes.len() <= 1 {
            return sorted_boxes.first().cloned().unwrap_or_default();
        }
        let total_size: usize = sorted_boxes.iter().map(|b| b.len()).sum();
        let target_size: usize = arity * (total_size / sorted_boxes.len());

        let mut groups: Vec<&[Vec<SortedBox>]> = Vec::new();
        let mut group_weight = 0;
        let mut start_index = 0;
        let mut end_index = 1;
        for sorted_box in sorted_boxes.iter() {
            let box_weight = sorted_box.len();
            if group_weight + box_weight <= target_size {
                end_index += 1;
                group_weight += box_weight;
            } else {
                groups.push(&sorted_boxes[start_index..end_index]);
                group_weight = 0;
                start_index = end_index;
                end_index += 1;
            }
        }
        if start_index < sorted_boxes.len() {
            groups.push(&sorted_boxes[start_index..sorted_boxes.len()]);
        }
        let next_boxes: Vec<Vec<SortedBox>> =
            groups.into_par_iter().map(|group| self.merge_k_sorted_boxes(group)).collect();
        self.merge_sorted_boxes(&next_boxes, arity)
    }
    fn merge_k_sorted_boxes(&self, boxes: &[Vec<SortedBox>]) -> Vec<SortedBox> {
        if boxes.len() == 1 {
            return boxes[0].clone();
        }
        let total_len: usize = boxes.iter().map(|b| b.len()).sum();
        let mut merged: Vec<SortedBox> = Vec::with_capacity(total_len);
        let mut cursors = vec![0; boxes.len()];
        for _ in 0..total_len {
            let mut min_addr = u64::MAX;
            let mut min_index = 0;
            for (i, box_ref) in boxes.iter().enumerate() {
                // we take the new min_index only if addr is less than min_addr, because the
                // boxes are sorted by step (time)
                if cursors[i] < box_ref.len() && box_ref[cursors[i]].addr < min_addr {
                    min_addr = box_ref[cursors[i]].addr;
                    min_index = i;
                }
            }
            merged.push(boxes[min_index][cursors[min_index]].clone());
            cursors[min_index] += 1;
        }
        merged
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
                        skip_rows,
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
        assert!(
            internal_rows < self.config.rows as u64,
            "internal_rows:{} < config.rows:{} ===> addr: {:#010X} addr_uses.first_step:{} last_step:{}",
            internal_rows,
            self.config.rows,
            addr * 8,
            addr_uses.first_step,
            self.last_step
        );
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
