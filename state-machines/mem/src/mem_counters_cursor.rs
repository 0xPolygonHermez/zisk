use rayon::{prelude::*, ThreadPoolBuilder};
use std::sync::Arc;

use crate::MemCounters;
use zisk_common::ChunkId;

pub struct MemCountersCursor {
    cursor_index: usize,
    cursor_count: usize,
    counters_count: usize,
    sorted_boxes: Vec<SortedBox>,
}

#[derive(Debug, Default, Clone)]
pub struct SortedBox {
    pub addr: u64,
    pub chunk: ChunkId,
    pub count: u32,
}

impl MemCountersCursor {
    pub fn new(counters: Arc<Vec<(ChunkId, &MemCounters)>>, addr_index: usize) -> Self {
        // let t_start = std::time::Instant::now();
        let counters_count = counters.len();
        let initial_sorted_boxes = Self::prepare(counters, addr_index);
        // let t_prepare = std::time::Instant::now();
        let sorted_boxes = Self::merge_sorted_boxes(&initial_sorted_boxes, 16);
        Self { counters_count, cursor_index: 0, cursor_count: sorted_boxes.len(), sorted_boxes }
        // let elapsed = std::time::Instant::now() - t_start;
        // let elapsed_prepare = t_prepare - t_start;
        // println!(
        //     "MemCountersCursor::new() elapsed: {} ms prepare: {} ms",
        //     elapsed.as_millis(),
        //     elapsed_prepare.as_millis()
        // );
    }
    #[inline(always)]
    pub fn init(&mut self) {
        self.cursor_index = 0;
        self.cursor_count = self.sorted_boxes.len();
    }
    #[inline(always)]
    pub fn end(&self) -> bool {
        self.cursor_index >= self.cursor_count
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.counters_count == 0
    }
    pub fn get_next(&mut self) -> (ChunkId, u32, u32) {
        let cursor = &self.sorted_boxes[self.cursor_index];
        self.cursor_index += 1;
        (cursor.chunk, cursor.addr as u32, cursor.count)
    }

    fn prepare(
        counters: Arc<Vec<(ChunkId, &MemCounters)>>,
        addr_index: usize,
    ) -> Vec<Vec<SortedBox>> {
        let pool = ThreadPoolBuilder::new().num_threads(16).build().unwrap();
        pool.install(|| {
            counters
                .par_iter()
                .map(|counter| {
                    // println!("SORT chunk {}", counter.0);
                    let addr_count = counter.1.addr_sorted[addr_index].len();
                    let mut counter_boxes: Vec<SortedBox> = Vec::with_capacity(addr_count);
                    for i_addr in 0..addr_count {
                        counter_boxes.push(SortedBox {
                            addr: counter.1.addr_sorted[addr_index][i_addr].0 as u64,
                            chunk: counter.0,
                            count: counter.1.addr_sorted[addr_index][i_addr].1,
                        });
                    }
                    counter_boxes
                })
                .collect()
        })
    }
    fn merge_sorted_boxes(sorted_boxes: &[Vec<SortedBox>], arity: usize) -> Vec<SortedBox> {
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
            groups.into_par_iter().map(Self::merge_k_sorted_boxes).collect();
        Self::merge_sorted_boxes(&next_boxes, arity)
    }
    fn merge_k_sorted_boxes(boxes: &[Vec<SortedBox>]) -> Vec<SortedBox> {
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
}
