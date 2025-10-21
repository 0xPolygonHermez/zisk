use core::panic;
use std::{collections::HashMap, sync::Arc};

use crate::{MemAlignCheckPoint, MemAlignCounters};
use crate::{MemAlignInstanceCounter, MemCounters};
use fields::Goldilocks;
use zisk_common::{ChunkId, Plan};
use zisk_pil::{
    MemAlignByteTrace, MemAlignReadByteTrace, MemAlignTrace, MemAlignWriteByteTrace,
    MEM_ALIGN_AIR_IDS, MEM_ALIGN_BYTE_AIR_IDS, MEM_ALIGN_READ_BYTE_AIR_IDS,
    MEM_ALIGN_WRITE_BYTE_AIR_IDS,
};

const ROWS_WRITE_BYTE: u32 = 3;
const ROWS_READ_BYTE: u32 = 2;
const WORSE_FRAGMENTATION: u32 = 4; // worse case fragmentation rows per full instance

// Base Columns cost by instance
const MEM_ALIGN_BCOLS: u32 = 56;
const MEM_ALIGN_READ_BYTE_BCOLS: u32 = 25;
const MEM_ALIGN_WRITE_BYTE_BCOLS: u32 = 32;
const MEM_ALIGN_BYTE_BCOLS: u32 = 32;

#[allow(dead_code)]
pub struct MemAlignPlanner<'a> {
    plans: Vec<Plan>,
    chunk_id: Option<ChunkId>,
    chunks: Vec<ChunkId>,
    check_points: HashMap<ChunkId, MemAlignCheckPoint>,
    full: MemAlignInstanceCounter,
    read_byte: MemAlignInstanceCounter,
    write_byte: MemAlignInstanceCounter,
    byte: MemAlignInstanceCounter,
    counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
}

impl<'a> MemAlignPlanner<'a> {
    pub fn new(counters: Arc<Vec<(ChunkId, &'a MemCounters)>>) -> Self {
        let full = MemAlignInstanceCounter::new(
            MEM_ALIGN_AIR_IDS[0],
            0,
            MemAlignTrace::<Goldilocks>::NUM_ROWS as u32,
            &[5, 3, 2, 2, 3],
            &[0, 1, 2, 3, 4],
        );

        let read_byte = MemAlignInstanceCounter::new(
            MEM_ALIGN_READ_BYTE_AIR_IDS[0],
            0,
            MemAlignReadByteTrace::<Goldilocks>::NUM_ROWS as u32,
            &[0, 0, 0, 1, 0],
            &[3],
        );

        let write_byte = MemAlignInstanceCounter::new(
            MEM_ALIGN_WRITE_BYTE_AIR_IDS[0],
            0,
            MemAlignWriteByteTrace::<Goldilocks>::NUM_ROWS as u32,
            &[0, 0, 0, 0, 1],
            &[4],
        );

        let byte = MemAlignInstanceCounter::new(
            MEM_ALIGN_BYTE_AIR_IDS[0],
            0,
            MemAlignByteTrace::<Goldilocks>::NUM_ROWS as u32,
            &[0, 0, 0, 1, 1],
            &[4, 3],
        );

        Self {
            plans: Vec::new(),
            chunk_id: None,
            chunks: Vec::new(),
            check_points: HashMap::new(),
            counters,
            read_byte,
            write_byte,
            byte,
            full,
        }
    }
    fn check_pendings(&self, pendings: &[u32; 5]) {
        if pendings.iter().any(|&x| x > 0) {
            println!(
                "[ReadByte] Instances:{}/{} Rows:{}/{} used:({:?})",
                self.read_byte.get_instances_available(),
                self.read_byte.get_instances(),
                self.read_byte.rows_available,
                self.read_byte.num_rows,
                self.read_byte.get_used()
            );
            println!(
                "[WriteByte] Instances:{}/{} Rows:{}/{} used:({:?})",
                self.write_byte.get_instances_available(),
                self.write_byte.get_instances(),
                self.write_byte.rows_available,
                self.write_byte.num_rows,
                self.write_byte.get_used()
            );
            println!(
                "[Byte] Instances:{}/{} Rows:{}/{} used:({:?})",
                self.byte.get_instances_available(),
                self.byte.get_instances(),
                self.byte.rows_available,
                self.byte.num_rows,
                self.byte.get_used()
            );
            println!(
                "[Full] Instances:{}/{} Rows:{}/{} used:({:?})",
                self.full.get_instances_available(),
                self.full.get_instances(),
                self.full.rows_available,
                self.full.num_rows,
                self.full.get_used()
            );
            println!("[Pending] (F5,F3,F2,RB,WB) {pendings:?}");
            panic!("Some counters are pending");
        }
    }
    pub fn align_plan(&mut self) {
        if self.counters.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }

        let count = self.counters.len();
        self.calculate_strategy();

        for index in 0..count {
            let chunk_id = self.counters[index].0;
            let totals = self.counters[index].1.to_array();
            self.align_plan_add_chunk(chunk_id, &totals);
        }
        self.close_instances();
        self.drain_all_plans();
    }
    fn align_plan_add_chunk(&mut self, chunk_id: ChunkId, totals: &[u32; 5]) {
        let mut pendings = *totals;
        self.read_byte.add_to_instance(chunk_id, totals, &mut pendings);
        self.write_byte.add_to_instance(chunk_id, totals, &mut pendings);
        self.byte.add_to_instance(chunk_id, totals, &mut pendings);
        self.full.add_to_instance(chunk_id, totals, &mut pendings);
        self.check_pendings(&pendings);
    }
    pub fn align_plan_from_counters(
        &mut self,
        full_rows: u32,
        read_byte: u32,
        write_byte: u32,
        counters: &[MemAlignCounters],
    ) {
        if counters.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }

        let count = counters.len();
        self.calculate_strategy_from_totals(full_rows, read_byte, write_byte);

        for counter in counters.iter().take(count) {
            let chunk_id = counter.chunk_id;
            let totals = counter.to_array();
            self.align_plan_add_chunk(ChunkId(chunk_id as usize), &totals);
        }
        self.close_instances();
        self.drain_all_plans();
    }

    fn close_instances(&mut self) {
        self.read_byte.close_instance();
        self.write_byte.close_instance();
        self.byte.close_instance();
        self.full.close_instance();
    }
    fn drain_all_plans(&mut self) {
        // Calculate total capacity needed
        let total_capacity: usize = self.read_byte.plans.len()
            + self.write_byte.plans.len()
            + self.byte.plans.len()
            + self.full.plans.len();

        self.plans = Vec::with_capacity(total_capacity);

        self.plans.append(&mut self.read_byte.plans);
        self.plans.append(&mut self.write_byte.plans);
        self.plans.append(&mut self.byte.plans);
        self.plans.append(&mut self.full.plans);
    }
    fn calculate_totals(&mut self) -> (u32, u32, u32) {
        let mut read_byte = 0;
        let mut write_byte = 0;
        let mut full_rows = 0;
        for counter in self.counters.iter() {
            let full = counter.1.mem_align_counters.full_2 * 2
                + counter.1.mem_align_counters.full_3 * 3
                + counter.1.mem_align_counters.full_5 * 5;
            full_rows += full;
            read_byte += counter.1.mem_align_counters.read_byte;
            write_byte += counter.1.mem_align_counters.write_byte;
        }
        (full_rows, read_byte, write_byte)
    }
    fn calculate_strategy_from_totals(&mut self, full_rows: u32, read_byte: u32, write_byte: u32) {
        let read_byte_instance_cost = MEM_ALIGN_READ_BYTE_BCOLS * self.read_byte.num_rows;
        let write_byte_instance_cost = MEM_ALIGN_WRITE_BYTE_BCOLS * self.write_byte.num_rows;
        let byte_instance_cost = MEM_ALIGN_BYTE_BCOLS * self.byte.num_rows;
        let full_instance_cost = MEM_ALIGN_BCOLS * self.full.num_rows;

        let mut byte_instances = 0;
        let mut read_byte_instances = read_byte / self.read_byte.num_rows;
        let mut write_byte_instances = write_byte / self.write_byte.num_rows;
        let mut full_instances = (full_rows / self.full.num_rows)
            + if (full_rows % self.full.num_rows) > 0 { 1 } else { 0 };

        let p_read_byte = read_byte % self.read_byte.num_rows;
        let p_write_byte = write_byte % self.write_byte.num_rows;

        // calculate the worse case of fragmentation at end of instance
        let fragmentation_rows = WORSE_FRAGMENTATION * full_instances;

        let max_full_free_rows = (full_instances * self.full.num_rows) - full_rows;

        // for security reasons, the worst case was that last 4 rows are lost.
        let full_free_rows = max_full_free_rows.saturating_sub(fragmentation_rows);

        let full_free_byte = full_free_rows / ROWS_WRITE_BYTE;
        let _strategy =
            if (ROWS_READ_BYTE * p_read_byte + ROWS_WRITE_BYTE * p_write_byte) <= full_free_rows {
                /* nothing, no need extra instances use free space on full mem_align */
                "+0"
            } else if (ROWS_WRITE_BYTE * p_write_byte) <= full_free_rows {
                // If all writes fit in the cache then only need one instance for pending reads.
                read_byte_instances += 1;
                "+read_byte"
            } else if (ROWS_READ_BYTE * p_read_byte) <= full_free_rows {
                // If all reads fit in the cache then only need one instance for pending writes.
                write_byte_instances += 1;
                "+write_byte"
            } else if (p_write_byte + p_read_byte) <= self.byte.num_rows {
                // If all pending reads and writes fit in read-write byte instance, we need one instance for both.
                byte_instances += 1;
                "+byte"
            } else if (p_read_byte + p_write_byte - full_free_byte) <= self.byte.num_rows
                && byte_instance_cost <= (read_byte_instance_cost + write_byte_instance_cost)
            {
                // at this point all reads no fit to full free rows, same for writes, and both no fit to byte instance.
                // but, a part of reads (uses less full rows) could fit in the full free rows. The rest of reads and all
                // pending writes fit in the byte instance (this is verified by if condition)
                byte_instances += 1;
                "+byte +0"
            } else if (p_read_byte * ROWS_READ_BYTE + p_write_byte * ROWS_WRITE_BYTE)
                < (full_free_rows + self.full.num_rows - WORSE_FRAGMENTATION)
                && full_instance_cost <= (read_byte_instance_cost + write_byte_instance_cost)
            {
                // if all pending reads and writes fit on last free rows plus one new full instance.
                full_instances += 1;
                "+full"
            } else {
                // if we need to add more than one instance for pending reads and writes, better use
                // two specific and cheaper instances.
                read_byte_instances += 1;
                write_byte_instances += 1;
                "+read_byte +write_byte"
            };
        self.byte.set_instances(byte_instances);
        self.read_byte.set_instances(read_byte_instances);
        self.write_byte.set_instances(write_byte_instances);
        self.full.set_instances(full_instances);
    }
    fn calculate_strategy(&mut self) {
        let (full_rows, read_byte, write_byte) = self.calculate_totals();
        self.calculate_strategy_from_totals(full_rows, read_byte, write_byte);
    }
    pub fn plan(&mut self) {
        self.align_plan();
    }
    pub fn collect_plans(&mut self) -> Vec<Plan> {
        std::mem::take(&mut self.plans)
    }
}
