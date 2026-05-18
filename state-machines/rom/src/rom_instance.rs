//! The `RomInstance` performs the witness computation based on the provided ROM execution plan
//!
//! It is responsible for computing witnesses for ROM-related execution plans,

use std::sync::{atomic::AtomicU64, Arc};

use crate::rom_counter::RomCounter;
use asm_runner::{AsmRHData, AsmRunnerRH};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx, TraceInfo};
use rayon::prelude::*;
use std::sync::Mutex;
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, BusId, CheckPoint, ChunkId, CounterStats, Instance, InstanceCtx, InstanceType,
    Metrics, PayloadType, ROM_BUS_ID,
};
use zisk_core::{ZiskRom, ROM_EXIT};
use zisk_pil::{MainTrace, RomTrace};

/// Per-emulator state held by a `RomInstance`. Mirrors `RomCounters` in `rom.rs`.
enum RomInstanceMode {
    /// Rust emulator path: counters indexed by physical address and an aggregated
    /// `CounterStats` populated after all chunks are collected.
    Rust { counter_stats: Mutex<Option<CounterStats>> },
    /// ASM emulator path: histogram delivered by the assembly runner.
    Asm { rh_data: Mutex<Option<AsmRunnerRH>> },
}

/// The `RomInstance` struct represents an instance to perform the witness computations for
/// ROM-related execution plans.
///
/// It encapsulates the `ZiskRom` and its associated context, and it interacts with
/// the `RomSM` to compute witnesses for the given execution plan.
pub struct RomInstance {
    /// Reference to the Zisk ROM.
    zisk_rom: Arc<ZiskRom>,

    /// The instance context.
    ictx: InstanceCtx,

    /// Shared program instruction counter for monitoring ROM operations.
    inst_count: Arc<Vec<AtomicU64>>,

    /// Per-emulator state.
    mode: RomInstanceMode,
}

impl RomInstance {
    /// Creates a new `RomInstance`.
    ///
    /// # Arguments
    /// * `zisk_rom` - An `Arc`-wrapped reference to the Zisk ROM.
    /// * `ictx` - The `InstanceCtx` associated with this instance.
    ///
    /// # Returns
    /// A new `RomInstance` instance initialized with the provided ROM and context.
    pub fn new_rust(
        zisk_rom: Arc<ZiskRom>,
        ictx: InstanceCtx,
        inst_count: Arc<Vec<AtomicU64>>,
    ) -> Self {
        Self {
            zisk_rom,
            ictx,
            inst_count,
            mode: RomInstanceMode::Rust { counter_stats: Mutex::new(None) },
        }
    }

    pub fn new_asm(
        zisk_rom: Arc<ZiskRom>,
        ictx: InstanceCtx,
        inst_count: Arc<Vec<AtomicU64>>,
        rh_data: AsmRunnerRH,
    ) -> Self {
        Self {
            zisk_rom,
            ictx,
            inst_count,
            mode: RomInstanceMode::Asm { rh_data: Mutex::new(Some(rh_data)) },
        }
    }

    pub fn skip_collector(&self) -> bool {
        match &self.mode {
            RomInstanceMode::Asm { .. } => true,
            RomInstanceMode::Rust { counter_stats, .. } => counter_stats.lock().unwrap().is_some(),
        }
    }

    pub fn build_rom_collector(&self, _chunk_id: ChunkId) -> Option<RomCollector> {
        match &self.mode {
            RomInstanceMode::Asm { .. } => None,
            RomInstanceMode::Rust { counter_stats, .. } => {
                let already_computed = counter_stats.lock().unwrap().is_some();
                if already_computed {
                    return None;
                }
                Some(RomCollector::new(already_computed, self.inst_count.clone()))
            }
        }
    }

    /// Builds the ROM air instance from aggregated Rust-emulator counters.
    fn compute_witness_from_counters<F: PrimeField64>(
        &self,
        counter_stats: &CounterStats,
        mut trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let main_trace_len = MainTrace::<()>::NUM_ROWS as u64;

        tracing::debug!("··· Creating Rom instance [{} rows]", RomTrace::<F>::NUM_ROWS);

        // For every instruction in the rom, fill its corresponding ROM trace
        for zib in self.zisk_rom.insts.values() {
            // Get the Zisk instruction
            let inst = &zib.i;

            // Calculate the multiplicity, i.e. the number of times this pc is used in this
            // execution
            let mut multiplicity: u64;
            multiplicity = counter_stats.inst_count[inst.index as usize]
                .load(std::sync::atomic::Ordering::Relaxed);
            if multiplicity == 0 {
                continue;
            }
            if inst.paddr == counter_stats.end_pc {
                multiplicity += main_trace_len - counter_stats.steps % main_trace_len;
            }

            let index = inst.index as usize;
            debug_assert!(
                 index < trace_buffer.len(),
                 "ROM trace index {} out of bounds for trace_buffer len {} (RomTrace::NUM_ROWS = {})",
                 index,
                 trace_buffer.len(),
                 RomTrace::<F>::NUM_ROWS
            );
            trace_buffer[index] = F::from_u64(multiplicity);
        }

        Ok(AirInstance::new(TraceInfo::new(
            RomTrace::<F>::AIRGROUP_ID,
            RomTrace::<F>::AIR_ID,
            1,
            RomTrace::<F>::NUM_ROWS,
            trace_buffer,
            false,
            false,
        )))
    }

    /// Builds the ROM air instance from the ASM-emulator histogram.
    fn compute_witness_from_asm<F: PrimeField64>(
        &self,
        asm_romh: &AsmRHData,
        mut trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        tracing::debug!("··· Creating Rom instance [{} rows]", RomTrace::<F>::NUM_ROWS);

        // Check that the provided histogram has at most as many entries as the ROM trace
        assert!(
            asm_romh.inst_count.len() <= RomTrace::<F>::NUM_ROWS,
            "The provided assembly histogram has {} entries, which exceeds the maximum supported by the Zisk PIL ROM trace ({} entries).  Please review zisk.pil and increase the ROM trace size accordingly.",
            asm_romh.inst_count.len(),
            RomTrace::<F>::NUM_ROWS
        );
        assert!(
            asm_romh.inst_count.len() <= trace_buffer.len(),
            "The provided assembly histogram has {} entries, but the trace buffer has only {} entries.",
            asm_romh.inst_count.len(),
            trace_buffer.len()
        );

        for (i, multiplicity) in asm_romh.inst_count.iter().enumerate() {
            if *multiplicity == 0 {
                continue;
            }
            trace_buffer[i] = F::from_u64(*multiplicity);
        }

        // Search for end instruction index
        let index = self.zisk_rom.get_instruction(ROM_EXIT).index as usize;
        assert!(
            index < trace_buffer.len(),
            "ROM trace index {} out of bounds for trace_buffer len {} (RomTrace::NUM_ROWS = {})",
            index,
            trace_buffer.len(),
            RomTrace::<F>::NUM_ROWS
        );
        assert!(
            F::is_one(&trace_buffer[index]),
            "The exit instruction should have been executed once in the assembly execution"
        );

        // Increment it as if it was executed the number of times needed to reach the end of the
        // main trace instance, i.e. we repeat the last instruction until the end of the instance
        let main_trace_len = MainTrace::<()>::NUM_ROWS as u64;
        trace_buffer[index] = F::from_u64(1 + main_trace_len - asm_romh.steps % main_trace_len);

        Ok(AirInstance::new(TraceInfo::new(
            RomTrace::<F>::AIRGROUP_ID,
            RomTrace::<F>::AIR_ID,
            1,
            RomTrace::<F>::NUM_ROWS,
            trace_buffer,
            false,
            false,
        )))
    }
}

impl<F: PrimeField64> Instance<F> for RomInstance {
    /// Computes the witness for the ROM execution plan.
    ///
    /// This method leverages the `RomSM` to generate an `AirInstance` based on the
    /// Zisk ROM and the provided execution plan.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    /// * `_collectors` - A vector of input collectors to process and collect data for witness,
    ///   unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        _packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        match &self.mode {
            // ASM path: borrow the histogram delivered by the assembly runner.
            RomInstanceMode::Asm { rh_data } => {
                let guard = rh_data.lock().unwrap();
                let rh = guard.as_ref().expect("rh_data not set on ASM RomInstance");
                Ok(Some(self.compute_witness_from_asm(&rh.asm_rowh_output, trace_buffer)?))
            }
            // Rust path: aggregate collector stats on first call, then build the trace.
            RomInstanceMode::Rust { counter_stats } => {
                if counter_stats.lock().unwrap().is_none() {
                    let collectors: Vec<_> = collectors
                        .into_iter()
                        .map(|(_, c)| c.as_any().downcast::<RomCollector>().unwrap())
                        .collect();

                    let mut stats = CounterStats::new(self.inst_count.clone());
                    for collector in collectors {
                        stats += &collector.rom_counter.counter_stats;
                    }
                    *counter_stats.lock().unwrap() = Some(stats);
                }

                Ok(Some(self.compute_witness_from_counters(
                    counter_stats.lock().unwrap().as_ref().unwrap(),
                    trace_buffer,
                )?))
            }
        }
    }

    fn reset(&self) {
        match &self.mode {
            // ASM mode: rh_data is source input from the assembly runner, not derived state.
            // `registry.rs` calls `reset()` before `compute_witness`, so clearing rh_data here
            // would drop the histogram we need.
            RomInstanceMode::Asm { .. } => {}
            RomInstanceMode::Rust { counter_stats } => {
                *counter_stats.lock().unwrap() = None;

                self.inst_count
                    .par_iter()
                    .for_each(|i| i.store(0, std::sync::atomic::Ordering::Relaxed));
            }
        }
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Memory
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, _: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        match &self.mode {
            RomInstanceMode::Asm { .. } => None,
            RomInstanceMode::Rust { counter_stats } => {
                let already_computed = counter_stats.lock().unwrap().is_some();
                if already_computed {
                    return None;
                }
                Some(Box::new(RomCollector::new(already_computed, self.inst_count.clone())))
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct RomCollector {
    /// Flag indicating if the table has been already computed.
    already_computed: bool,

    /// Execution statistics counter for the ROM.
    pub rom_counter: RomCounter,
}

impl RomCollector {
    /// Creates a new instance of `RomCounter`.
    ///
    /// # Returns
    /// A new `RomCounter` instance.
    pub fn new(computed: bool, inst_count: Arc<Vec<AtomicU64>>) -> Self {
        let rom_counter = RomCounter::new(inst_count);
        Self { already_computed: computed, rom_counter }
    }

    /// Processes data received on the bus, updating ROM metrics.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        debug_assert!(*bus_id == ROM_BUS_ID);

        if !self.already_computed {
            self.rom_counter.measure(data);
        }

        true
    }
}

impl BusDevice<u64> for RomCollector {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
