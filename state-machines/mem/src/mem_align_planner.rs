use core::panic;
use std::{collections::HashMap, sync::Arc};

use crate::{MemAlignInstanceCounter, MemCounters, MemPlanCalculator};
use mem_common::MemAlignCheckPoint;
use zisk_common::{ChunkId, Plan};
use zisk_pil::{
    MemAlignByteTrace, MemAlignReadByteTrace, MemAlignTrace, MemAlignWriteByteTrace,
    MEM_ALIGN_AIR_IDS, MEM_ALIGN_BYTE_AIR_IDS, MEM_ALIGN_READ_BYTE_AIR_IDS,
    MEM_ALIGN_WRITE_BYTE_AIR_IDS,
};

const ROWS_WRITE_BYTE: u32 = 3;
const ROWS_READ_BYTE: u32 = 2;
const WORSE_FRAGMENTATION: u32 = 4; // worse case fragmentation rows per full instance

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
            MemAlignTrace::<usize>::NUM_ROWS as u32,
            [5, 3, 2, 2, 3],
        );

        let read_byte = MemAlignInstanceCounter::new(
            MEM_ALIGN_READ_BYTE_AIR_IDS[0],
            0,
            MemAlignReadByteTrace::<usize>::NUM_ROWS as u32,
            [0, 0, 0, 1, 0],
        );

        let write_byte = MemAlignInstanceCounter::new(
            MEM_ALIGN_WRITE_BYTE_AIR_IDS[0],
            0,
            MemAlignWriteByteTrace::<usize>::NUM_ROWS as u32,
            [0, 0, 0, 0, 1],
        );

        let byte = MemAlignInstanceCounter::new(
            MEM_ALIGN_BYTE_AIR_IDS[0],
            0,
            MemAlignByteTrace::<usize>::NUM_ROWS as u32,
            [0, 0, 0, 1, 1],
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
    pub fn align_plan(&mut self) {
        if self.counters.is_empty() {
            panic!("MemPlanner::plan() No metrics found");
        }

        let count = self.counters.len();
        self.calculate_strategy();

        for index in 0..count {
            let chunk_id = self.counters[index].0;
            let totals = self.counters[index].1.to_array();
            let mut pendings = totals;
            self.read_byte.add_to_instance(chunk_id, &totals, &mut pendings);
            self.write_byte.add_to_instance(chunk_id, &totals, &mut pendings);
            self.byte.add_to_instance(chunk_id, &totals, &mut pendings);
            self.full.add_to_instance(chunk_id, &totals, &mut pendings);
            if pendings.iter().any(|&x| x > 0) {
                println!(
                    "[ReadByte] Instances:{}/{} Rows:{}/{}",
                    self.read_byte.get_instances_available(),
                    self.read_byte.get_instances(),
                    self.read_byte.rows_available,
                    self.read_byte.num_rows
                );
                println!(
                    "[WriteByte] Instances:{}/{} Rows:{}/{}",
                    self.write_byte.get_instances_available(),
                    self.write_byte.get_instances(),
                    self.write_byte.rows_available,
                    self.write_byte.num_rows
                );
                println!(
                    "[Byte] Instances:{}/{} Rows:{}/{}",
                    self.byte.get_instances_available(),
                    self.byte.get_instances(),
                    self.byte.rows_available,
                    self.byte.num_rows
                );
                println!(
                    "[Full] Instances:{}/{} Rows:{}/{}",
                    self.full.get_instances_available(),
                    self.full.get_instances(),
                    self.full.rows_available,
                    self.full.num_rows
                );
                println!("[Pending] (F5,F3,F2,RB,WB) {pendings:?}");
                panic!("Some counters are pending");
            }
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
    fn calculate_strategy(&mut self) {
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

        let full_free_byte_reads = full_free_rows / ROWS_READ_BYTE;
        if (ROWS_READ_BYTE * p_read_byte + ROWS_WRITE_BYTE * p_write_byte) <= full_free_rows {
            /* nothing, no need extra instances use free space on full mem_align */
            println!("MEM_ALIGN_STRATEGY: No extra instances needed");
        } else if (ROWS_WRITE_BYTE * p_write_byte) <= full_free_rows {
            // If all writes fit in the cache then only need one instance for pending reads.
            println!("MEM_ALIGN_STRATEGY: All pending writes fit on full free rows, one more read byte instance");
            read_byte_instances += 1;
        } else if (ROWS_READ_BYTE * p_read_byte) <= full_free_rows {
            // If all reads fit in the cache then only need one instance for pending writes.
            println!("MEM_ALIGN_STRATEGY: All pending reads fit on full free rows, one more write byte instance");
            write_byte_instances += 1;
        } else if (p_write_byte + p_read_byte) <= self.byte.num_rows {
            // If all pending reads and writes fit in read-write byte instance, we need one instance for both.
            println!("MEM_ALIGN_STRATEGY: All pending reads and writes fit on one byte instance");
            byte_instances += 1;
        } else if (p_read_byte + p_write_byte - full_free_byte_reads) <= self.byte.num_rows {
            // at this point all reads no fit to full free rows, same for writes, and both no fit to byte instance.
            // but, a part of reads (uses less full rows) could fit in the full free rows. The rest of reads and all
            // pending writes fit in the byte instance (this is verified by if condition)
            println!("MEM_ALIGN_STRATEGY: All pending reads and writes fit on one byte instance (after put some reads on full free rows)");
            byte_instances += 1;
        } else if (p_read_byte * ROWS_READ_BYTE + p_write_byte * ROWS_WRITE_BYTE
            - full_free_byte_reads)
            <= self.full.num_rows
        {
            // if all pending reads and writes fit on last free rows plus one new full instance.
            println!("MEM_ALIGN_STRATEGY: All pending reads and writes fit on new full instance (after put some reads on full free rows)");
            full_instances += 1;
        } else {
            // if we need to add more than one instance for pending reads and writes, better use
            // two specific and cheaper instances.
            println!("MEM_ALIGN_STRATEGY: All pending reads and writes fit needs two instances");
            read_byte_instances += 1;
            write_byte_instances += 1;
        }
        self.byte.set_instances(byte_instances);
        self.read_byte.set_instances(read_byte_instances);
        self.write_byte.set_instances(write_byte_instances);
        self.full.set_instances(full_instances);
    }
}

impl MemPlanCalculator for MemAlignPlanner<'_> {
    fn plan(&mut self) {
        self.align_plan();
    }
    fn collect_plans(&mut self) -> Vec<Plan> {
        std::mem::take(&mut self.plans)
    }
}
