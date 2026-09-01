//! Sizing and filling the mem-align instances.
//!
//! # The airs
//!
//! Five kinds of operation are counted — `full_5`, `full_3`, `full_2`, `read_byte` and `write_byte` —
//! and each air proves a subset of them at a row cost of its own:
//!
//! | air                                    | proves                | rows per operation      |
//! |----------------------------------------|-----------------------|-------------------------|
//! | `MemAlign` / `MemAlignLarge`           | everything            | 5 / 3 / 2, 2 read, 3 write |
//! | `MemAlignByte` / `MemAlignByteLarge`   | read_byte, write_byte | 1                       |
//! | `MemAlignReadByte` / `…ReadByteLarge`  | read_byte             | 1                       |
//! | `MemAlignWriteByte`                    | write_byte            | 1                       |
//!
//! The byte airs are the cheap home for a byte operation — one row instead of two or three — but the
//! `MemAlign` airs prove it too, which is what lets a handful of byte operations ride in the room the
//! full ones already paid for instead of opening an instance of their own.
//!
//! # The strategy
//!
//! [`zisk_common::select_airs`] decides, under the shared criterion (fewest instances first, least
//! area to break a tie), which air each kind goes to and how many instances of each are granted. The
//! assignment is then written into the fill order and the per-air costs, so [`MemAlignInstanceCounter`]
//! only ever offers a kind to the air it was assigned to — which is what keeps the fill from
//! consuming room the sizing had promised to another kind.
//!
//! An operation never straddles two instances, so a `MemAlign` instance can waste up to
//! [`WORSE_FRAGMENTATION`] rows in its tail. That is taken off the height the sizing sees rather than
//! patched afterwards, so the granted instances always hold what was routed to them.

use core::panic;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    sync::Arc,
};

use crate::{MemAlignCheckPoint, MemAlignCounters};
use crate::{MemAlignInstanceCounter, MemCounters};
use proofman_fields::Goldilocks;
use zisk_common::{select_airs, AirChoice, ChunkId, Plan};
use zisk_pil::{
    MemAlignByteLargeTrace, MemAlignByteTrace, MemAlignLargeTrace, MemAlignReadByteLargeTrace,
    MemAlignReadByteTrace, MemAlignTrace, MemAlignWriteByteTrace,
};

const ROWS_WRITE_BYTE: u32 = 3;
const ROWS_READ_BYTE: u32 = 2;

/// Rows a `MemAlign` instance can lose in its tail: an operation takes up to five rows and cannot
/// straddle two instances, so up to four are left unusable.
const WORSE_FRAGMENTATION: u32 = 4;

/// Kinds of operation the strategy places, in the order [`MemCounters::to_array`] reports them.
mod kind {
    /// Operations taking five rows in a `MemAlign` air.
    pub const FULL_5: usize = 0;
    /// Operations taking three rows in a `MemAlign` air.
    pub const FULL_3: usize = 1;
    /// Operations taking two rows in a `MemAlign` air.
    pub const FULL_2: usize = 2;
    /// Unaligned byte reads.
    pub const READ_BYTE: usize = 3;
    /// Unaligned byte writes.
    pub const WRITE_BYTE: usize = 4;
    /// Number of kinds.
    pub const COUNT: usize = 5;
}

/// Airs of the family, in the order the strategy and the fill both use: the specialised and tallest
/// first, so the cheap homes fill before the general air is reached.
mod air {
    /// `MemAlignReadByteLarge`.
    pub const READ_BYTE_LARGE: usize = 0;
    /// `MemAlignReadByte`.
    pub const READ_BYTE: usize = 1;
    /// `MemAlignWriteByte`.
    pub const WRITE_BYTE: usize = 2;
    /// `MemAlignByteLarge`.
    pub const BYTE_LARGE: usize = 3;
    /// `MemAlignByte`.
    pub const BYTE: usize = 4;
    /// `MemAlignLarge`.
    pub const FULL_LARGE: usize = 5;
    /// `MemAlign`.
    pub const FULL: usize = 6;
    /// Number of airs.
    pub const COUNT: usize = 7;
}

/// Rows one operation of each kind takes in each air, or `0` where the air cannot prove the kind.
///
/// This is both the strategy's cost table and the counters' capability mask: a zero cost is what tells
/// [`MemAlignInstanceCounter`] the air does not support the kind.
const ROW_COST: [[u32; kind::COUNT]; air::COUNT] = [
    [0, 0, 0, 1, 0],                            // MemAlignReadByteLarge
    [0, 0, 0, 1, 0],                            // MemAlignReadByte
    [0, 0, 0, 0, 1],                            // MemAlignWriteByte
    [0, 0, 0, 1, 1],                            // MemAlignByteLarge
    [0, 0, 0, 1, 1],                            // MemAlignByte
    [5, 3, 2, ROWS_READ_BYTE, ROWS_WRITE_BYTE], // MemAlignLarge
    [5, 3, 2, ROWS_READ_BYTE, ROWS_WRITE_BYTE], // MemAlign
];

/// The airs the strategy chooses between, in [`air`] order.
fn air_choices() -> [AirChoice; air::COUNT] {
    // A full operation cannot straddle two instances, so the tail of a `MemAlign` instance may be
    // unusable. Taking it off the height here is what keeps the granted instances able to hold what
    // the sizing routed to them; the byte airs prove one-row operations and never fragment.
    let full = |airgroup_id, air_id, rows: usize| {
        let mut choice = AirChoice::new(airgroup_id, air_id, rows);
        choice.rows -= WORSE_FRAGMENTATION as u64;
        choice
    };
    [
        AirChoice::new(
            MemAlignReadByteLargeTrace::<()>::AIRGROUP_ID,
            MemAlignReadByteLargeTrace::<()>::AIR_ID,
            MemAlignReadByteLargeTrace::<()>::NUM_ROWS,
        ),
        AirChoice::new(
            MemAlignReadByteTrace::<()>::AIRGROUP_ID,
            MemAlignReadByteTrace::<()>::AIR_ID,
            MemAlignReadByteTrace::<()>::NUM_ROWS,
        ),
        AirChoice::new(
            MemAlignWriteByteTrace::<()>::AIRGROUP_ID,
            MemAlignWriteByteTrace::<()>::AIR_ID,
            MemAlignWriteByteTrace::<()>::NUM_ROWS,
        ),
        AirChoice::new(
            MemAlignByteLargeTrace::<()>::AIRGROUP_ID,
            MemAlignByteLargeTrace::<()>::AIR_ID,
            MemAlignByteLargeTrace::<()>::NUM_ROWS,
        ),
        AirChoice::new(
            MemAlignByteTrace::<()>::AIRGROUP_ID,
            MemAlignByteTrace::<()>::AIR_ID,
            MemAlignByteTrace::<()>::NUM_ROWS,
        ),
        full(
            MemAlignLargeTrace::<()>::AIRGROUP_ID,
            MemAlignLargeTrace::<()>::AIR_ID,
            MemAlignLargeTrace::<()>::NUM_ROWS,
        ),
        full(
            MemAlignTrace::<()>::AIRGROUP_ID,
            MemAlignTrace::<()>::AIR_ID,
            MemAlignTrace::<()>::NUM_ROWS,
        ),
    ]
}

/// One instance counter per air, in [`air`] order.
type Counters = [MemAlignInstanceCounter; air::COUNT];

#[allow(dead_code)]
pub struct MemAlignPlanner<'a> {
    plans: Vec<Plan>,
    chunk_id: Option<ChunkId>,
    chunks: Vec<ChunkId>,
    check_points: HashMap<ChunkId, MemAlignCheckPoint>,

    /// One counter per air, in [`air`] order: the strategy grants each its instances and the fill
    /// then walks them in that order, so the specialised airs take what they were assigned before
    /// the general one is reached.
    counters_by_air: Counters,

    counters: Arc<Vec<(ChunkId, &'a MemCounters)>>,
}

impl<'a> MemAlignPlanner<'a> {
    pub fn new(counters: Arc<Vec<(ChunkId, &'a MemCounters)>>) -> Self {
        let choices = air_choices();
        let heights = [
            MemAlignReadByteLargeTrace::<Goldilocks>::NUM_ROWS as u32,
            MemAlignReadByteTrace::<Goldilocks>::NUM_ROWS as u32,
            MemAlignWriteByteTrace::<Goldilocks>::NUM_ROWS as u32,
            MemAlignByteLargeTrace::<Goldilocks>::NUM_ROWS as u32,
            MemAlignByteTrace::<Goldilocks>::NUM_ROWS as u32,
            MemAlignLargeTrace::<Goldilocks>::NUM_ROWS as u32,
            MemAlignTrace::<Goldilocks>::NUM_ROWS as u32,
        ];

        // The counters start with no kind enabled; `set_strategy` turns on exactly the ones the
        // sizing assigned to each air, which is what keeps the fill from consuming room promised
        // elsewhere. The height is the air's real one — the fragmentation allowance only shrinks
        // what the sizing counts on, never what a filled instance may use.
        let counters_by_air = std::array::from_fn(|a| {
            MemAlignInstanceCounter::new(choices[a].air_id, 0, heights[a], &[0; kind::COUNT], &[])
        });

        Self {
            plans: Vec::new(),
            chunk_id: None,
            chunks: Vec::new(),
            check_points: HashMap::new(),
            counters,
            counters_by_air,
        }
    }

    /// Result indicating success or an IO error
    pub fn save_counters_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        for (chunk_id, mem_counters) in self.counters.as_ref() {
            let mc = &mem_counters.mem_align_counters;
            writeln!(
                writer,
                "{} {} {} {} {} {}",
                chunk_id.0, mc.full_5, mc.full_3, mc.full_2, mc.read_byte, mc.write_byte
            )?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Loads counters from a file and calculates totals for use with align_plan_from_counters.
    /// Returns the loaded counters and calculated totals (full_rows, read_byte, write_byte).
    ///
    /// # Parameters
    /// - `path`: Path to the file containing saved counters
    ///
    /// # Returns
    /// A tuple with (counters, full_rows, read_byte, write_byte)
    pub fn load_counters_from_file<P: AsRef<Path>>(
        path: P,
    ) -> std::io::Result<(Vec<MemAlignCounters>, u32, u32, u32)> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut counters: Vec<MemAlignCounters> = Vec::new();
        let mut full_rows = 0;
        let mut read_byte = 0;
        let mut write_byte = 0;

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 6 {
                continue;
            }

            let counter = MemAlignCounters {
                chunk_id: parts[0].parse().unwrap_or(0),
                full_5: parts[1].parse().unwrap_or(0),
                full_3: parts[2].parse().unwrap_or(0),
                full_2: parts[3].parse().unwrap_or(0),
                read_byte: parts[4].parse().unwrap_or(0),
                write_byte: parts[5].parse().unwrap_or(0),
            };

            full_rows += counter.full_2 * 2 + counter.full_3 * 3 + counter.full_5 * 5;
            read_byte += counter.read_byte;
            write_byte += counter.write_byte;

            counters.push(counter);
        }

        Ok((counters, full_rows, read_byte, write_byte))
    }

    fn check_pendings(&self, pendings: &[u32; kind::COUNT]) {
        if pendings.iter().all(|&x| x == 0) {
            return;
        }
        for (a, counter) in self.counters_by_air.iter().enumerate() {
            println!(
                "[air {a}] Instances:{}/{} Rows:{}/{} used:({:?})",
                counter.get_instances_available(),
                counter.get_instances(),
                counter.rows_available,
                counter.num_rows,
                counter.get_used()
            );
        }
        println!("[Pending] (F5,F3,F2,RB,WB) {pendings:?}");
        let _ = self.save_counters_to_file("tmp/mem_align_counters_crash.txt");
        panic!("Some counters are pending");
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

    fn align_plan_add_chunk(&mut self, chunk_id: ChunkId, totals: &[u32; kind::COUNT]) {
        let mut pendings = *totals;
        for counter in self.counters_by_air.iter_mut() {
            counter.add_to_instance(chunk_id, totals, &mut pendings);
        }
        self.check_pendings(&pendings);
    }

    pub fn align_plan_from_counters(
        &mut self,
        full_rows: u32,
        read_byte: u32,
        write_byte: u32,
        counters: &[MemAlignCounters],
    ) {
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
        for counter in self.counters_by_air.iter_mut() {
            counter.close_instance();
        }
    }

    fn drain_all_plans(&mut self) {
        let total_capacity: usize = self.counters_by_air.iter().map(|c| c.plans.len()).sum();
        self.plans = Vec::with_capacity(total_capacity);
        for counter in self.counters_by_air.iter_mut() {
            self.plans.append(&mut counter.plans);
        }
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

    /// Decides which air proves each kind and how many instances of each are granted, then writes
    /// that decision into the counters so the fill follows it.
    ///
    /// The full operations are counted in rows already (`full_rows`), so they enter the sizing as a
    /// single kind that only the `MemAlign` airs can prove; the byte kinds carry their own row cost
    /// per air, which is what makes riding in the `MemAlign` room the dearer option per operation and
    /// yet the better one whenever it spares an instance.
    fn calculate_strategy_from_totals(&mut self, full_rows: u32, read_byte: u32, write_byte: u32) {
        let choices = air_choices();

        // Rows each kind takes in each air able to prove it. The three full kinds are already summed
        // into `full_rows`, so they share one entry and are routed together.
        let options = |ops: u32, k: usize| -> Vec<(usize, u64)> {
            (0..air::COUNT)
                .filter(|&a| ROW_COST[a][k] != 0)
                .map(|a| (a, ops as u64 * ROW_COST[a][k] as u64))
                .collect()
        };
        let kinds = vec![
            (0..air::COUNT)
                .filter(|&a| ROW_COST[a][kind::FULL_5] != 0)
                .map(|a| (a, full_rows as u64))
                .collect::<Vec<_>>(),
            options(read_byte, kind::READ_BYTE),
            options(write_byte, kind::WRITE_BYTE),
        ];

        let selection = select_airs(&kinds, &choices);

        // Turn on, in each air, exactly the kinds routed to it. A kind with no operations is left off
        // everywhere: enabling it would let the fill hand rows to an air the sizing never counted.
        let mut costs = [[0u32; kind::COUNT]; air::COUNT];
        if full_rows > 0 {
            let a = selection.assignment[0];
            for k in [kind::FULL_5, kind::FULL_3, kind::FULL_2] {
                costs[a][k] = ROW_COST[a][k];
            }
        }
        if read_byte > 0 {
            let a = selection.assignment[1];
            costs[a][kind::READ_BYTE] = ROW_COST[a][kind::READ_BYTE];
        }
        if write_byte > 0 {
            let a = selection.assignment[2];
            costs[a][kind::WRITE_BYTE] = ROW_COST[a][kind::WRITE_BYTE];
        }

        for (a, counter) in self.counters_by_air.iter_mut().enumerate() {
            counter.set_instances(selection.instances[a] as u32);
            // The kinds an air proves, dearest first, so the rows it has go to the operations that
            // would cost the most elsewhere.
            let mut order: Vec<usize> = (0..kind::COUNT).filter(|&k| costs[a][k] != 0).collect();
            order.sort_by_key(|&k| std::cmp::Reverse(costs[a][k]));
            counter.set_costs(&costs[a]);
            counter.update_order(&order);
        }

        tracing::debug!(
            "··· MemAlign instances: read_byte_large={} read_byte={} write_byte={} byte_large={} \
             byte={} full_large={} full={}",
            selection.instances[air::READ_BYTE_LARGE],
            selection.instances[air::READ_BYTE],
            selection.instances[air::WRITE_BYTE],
            selection.instances[air::BYTE_LARGE],
            selection.instances[air::BYTE],
            selection.instances[air::FULL_LARGE],
            selection.instances[air::FULL],
        );
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
