//! The `RomInstance` performs the witness computation based on the provided ROM execution plan.

use std::sync::{atomic::AtomicU64, Arc};

use crate::rom_counter::RomCounter;
use crate::{RomError, RomResult};
use asm_runner::{AsmRHData, AsmRunnerRH};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanError, ProofmanResult, SetupCtx, TraceInfo};
use rayon::prelude::*;
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, BusId, CheckPoint, ChunkId, CounterStats, Instance, InstanceCtx, InstanceType,
    Metrics, PayloadType, ROM_BUS_ID,
};
use zisk_core::{ZiskRom, ROM_EXIT};
use zisk_pil::{MainTrace, RomTrace};

/// Per-emulator state held by a `RomInstance`. Each variant owns exactly the data
/// its execution path needs and implements its own behaviour.
enum RomInstanceMode {
    Rust(RustState),
    Asm(AsmState),
}

/// State for the Rust-emulator path. Per-chunk collectors write into the shared
/// `inst_count` atomics; the trait `compute_witness` aggregates them once per cycle.
struct RustState {
    inst_count: Arc<Vec<AtomicU64>>,
}

/// State for the ASM-emulator path: histogram delivered by the assembly runner,
/// consumed directly when computing the witness.
struct AsmState {
    rh_data: AsmRunnerRH,
}

impl RustState {
    fn new(inst_count: Arc<Vec<AtomicU64>>) -> Self {
        Self { inst_count }
    }

    fn build_collector(&self) -> RomCollector {
        RomCollector::new(self.inst_count.clone())
    }

    /// Merges per-chunk collector state into a single `CounterStats`.
    fn aggregate_stats(
        &self,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
    ) -> RomResult<CounterStats> {
        let mut stats = CounterStats::new(self.inst_count.clone());
        for (_, collector) in collectors {
            let collector = collector
                .as_any()
                .downcast::<RomCollector>()
                .map_err(|_| RomError::BadCollectorType)?;
            stats += &collector.rom_counter.counter_stats;
        }
        Ok(stats)
    }

    fn reset(&self) {
        self.inst_count.par_iter().for_each(|i| i.store(0, std::sync::atomic::Ordering::Relaxed));
    }
}

impl AsmState {
    fn new(rh_data: AsmRunnerRH) -> Self {
        Self { rh_data }
    }

    /// Borrowed view of the assembly histogram. Keeps the wrapping `AsmRunnerRH`'s
    /// shape private to this module.
    fn histogram(&self) -> &AsmRHData {
        &self.rh_data.asm_rowh_output
    }
}

/// The `RomInstance` struct represents an instance to perform the witness computations for
/// ROM-related execution plans.
pub struct RomInstance {
    /// Reference to the Zisk ROM.
    zisk_rom: Arc<ZiskRom>,

    /// The instance context.
    ictx: InstanceCtx,

    /// Per-emulator state.
    mode: RomInstanceMode,
}

impl RomInstance {
    /// Creates a `RomInstance` for the Rust emulator path.
    pub fn new_rust(
        zisk_rom: Arc<ZiskRom>,
        ictx: InstanceCtx,
        inst_count: Arc<Vec<AtomicU64>>,
    ) -> Self {
        Self { zisk_rom, ictx, mode: RomInstanceMode::Rust(RustState::new(inst_count)) }
    }

    /// Creates a `RomInstance` for the ASM emulator path.
    pub fn new_asm(zisk_rom: Arc<ZiskRom>, ictx: InstanceCtx, rh_data: AsmRunnerRH) -> Self {
        Self { zisk_rom, ictx, mode: RomInstanceMode::Asm(AsmState::new(rh_data)) }
    }

    /// Returns true when this instance produces its witness without collecting bus data
    /// (currently only the ASM-emulator path).
    pub fn skip_collector(&self) -> bool {
        matches!(self.mode, RomInstanceMode::Asm(_))
    }

    /// Builds the per-chunk bus collector for this instance, or `None` in ASM mode where
    /// the witness comes from the assembly histogram instead.
    pub fn build_rom_collector(&self, _: ChunkId) -> Option<RomCollector> {
        match &self.mode {
            RomInstanceMode::Asm(_) => None,
            RomInstanceMode::Rust(r) => Some(r.build_collector()),
        }
    }

    /// Builds the ROM air instance from aggregated Rust-emulator counters.
    fn compute_witness_from_rust<F: PrimeField64>(
        zisk_rom: &ZiskRom,
        counter_stats: &CounterStats,
        mut trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let main_trace_len = MainTrace::<()>::NUM_ROWS as u64;

        // For every instruction in the rom, fill its corresponding ROM trace
        for zib in zisk_rom.insts.values() {
            // Get the Zisk instruction
            let inst = &zib.i;

            // Calculate the multiplicity, i.e. the number of times this pc is used in this
            // execution
            let mut multiplicity = counter_stats.inst_count[inst.index as usize]
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

        Self::build_air_instance(trace_buffer)
    }

    /// Builds the ROM air instance from the ASM-emulator histogram.
    fn compute_witness_from_asm<F: PrimeField64>(
        zisk_rom: &ZiskRom,
        asm_romh: &AsmRHData,
        mut trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
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
        let index = zisk_rom.get_instruction(ROM_EXIT).index as usize;
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

        // Increment as if executed the number of times needed to reach the end of the main trace
        // instance, i.e. repeat the last instruction until the end of the instance.
        let main_trace_len = MainTrace::<()>::NUM_ROWS as u64;
        trace_buffer[index] = F::from_u64(1 + main_trace_len - asm_romh.steps % main_trace_len);

        Self::build_air_instance(trace_buffer)
    }

    /// Wraps the filled `trace_buffer` in the ROM `AirInstance` expected by the proof pipeline.
    fn build_air_instance<F: PrimeField64>(trace_buffer: Vec<F>) -> AirInstance<F> {
        AirInstance::new(TraceInfo::new(
            RomTrace::<F>::AIRGROUP_ID,
            RomTrace::<F>::AIR_ID,
            1,
            RomTrace::<F>::NUM_ROWS,
            trace_buffer,
            false,
            false,
        ))
    }
}

impl<F: PrimeField64> Instance<F> for RomInstance {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        _packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        tracing::debug!("··· Creating Rom instance [{} rows]", RomTrace::<F>::NUM_ROWS);

        let air = match &self.mode {
            RomInstanceMode::Asm(a) => {
                Self::compute_witness_from_asm(&self.zisk_rom, a.histogram(), trace_buffer)
            }
            RomInstanceMode::Rust(r) => {
                let stats = r
                    .aggregate_stats(collectors)
                    .map_err(|e| ProofmanError::InvalidParameters(e.to_string()))?;
                Self::compute_witness_from_rust(&self.zisk_rom, &stats, trace_buffer)
            }
        };
        Ok(Some(air))
    }

    fn reset(&self) {
        match &self.mode {
            // ASM mode: rh_data is source input from the assembly runner, not derived state.
            // `registry.rs` calls `reset()` before `compute_witness`, so clearing rh_data here
            // would drop the histogram we need.
            RomInstanceMode::Asm(_) => {}
            RomInstanceMode::Rust(r) => r.reset(),
        }
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Memory
    }

    fn build_inputs_collector(&self, _: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        match &self.mode {
            RomInstanceMode::Asm(_) => None,
            RomInstanceMode::Rust(r) => Some(Box::new(r.build_collector())),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// `BusDevice` adapter that forwards ROM-bus traffic into the underlying counter.
pub struct RomCollector {
    /// Underlying counter that accumulates per-instruction execution counts.
    pub(crate) rom_counter: RomCounter,
}

impl RomCollector {
    /// Creates a new `RomCollector` backed by the shared `inst_count` atomics.
    pub(crate) fn new(inst_count: Arc<Vec<AtomicU64>>) -> Self {
        Self { rom_counter: RomCounter::new(inst_count) }
    }

    /// Processes data received on the bus, updating ROM metrics.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        debug_assert!(*bus_id == ROM_BUS_ID);
        self.rom_counter.measure(data);
        true
    }
}

impl BusDevice<u64> for RomCollector {
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asm_runner::{AsmRHData, AsmRunnerRH};
    use fields::Goldilocks;
    use std::sync::atomic::{AtomicU64, Ordering};
    use zisk_common::Plan;
    use zisk_core::{ZiskInst, ZiskInstBuilder};

    type F = Goldilocks;

    fn dummy_ictx() -> InstanceCtx {
        InstanceCtx::new(0, Plan::new(0, 0, None, InstanceType::Instance, CheckPoint::None, None))
    }

    fn asm_runner_rh_empty() -> AsmRunnerRH {
        AsmRunnerRH::new(AsmRHData::new(0, vec![]))
    }

    /// Builds a ZiskRom with `n` instructions placed at `min_program_pc + 4*i`, each
    /// carrying `.index = i` (which is the column the witness pipeline indexes by).
    fn rom_with_indexed_insts(min_program_pc: u64, n: usize) -> ZiskRom {
        let mut rom = ZiskRom { min_program_pc, ..Default::default() };
        for i in 0..n {
            let paddr = min_program_pc + 4 * i as u64;
            let mut zib = ZiskInstBuilder::new(paddr);
            zib.i.index = i as u64;
            rom.insts.insert(paddr, zib);
        }
        rom
    }

    fn atomics_from(counts: &[u64]) -> Arc<Vec<AtomicU64>> {
        Arc::new(counts.iter().map(|&c| AtomicU64::new(c)).collect())
    }

    #[test]
    fn build_air_instance_sets_rom_air_metadata() {
        let air = RomInstance::build_air_instance::<F>(vec![F::from_u64(0); 10]);
        assert_eq!(air.airgroup_id, RomTrace::<F>::AIRGROUP_ID);
        assert_eq!(air.air_id, RomTrace::<F>::AIR_ID);
        assert_eq!(air.num_rows, RomTrace::<F>::NUM_ROWS);
        assert_eq!(air.trace.len(), 10);
    }

    #[test]
    fn from_rust_writes_each_multiplicity_at_its_index() {
        let rom = rom_with_indexed_insts(0x8000_0000, 3);
        let stats = CounterStats {
            inst_count: atomics_from(&[5, 0, 10]),
            end_pc: 0xFFFF_FFFF, // does not match any inst → no end-of-trace bump
            steps: 0,
        };

        let air =
            RomInstance::compute_witness_from_rust::<F>(&rom, &stats, vec![F::from_u64(0); 10]);

        assert_eq!(air.trace[0], F::from_u64(5));
        assert_eq!(air.trace[1], F::from_u64(0)); // multiplicity==0 path skips the write
        assert_eq!(air.trace[2], F::from_u64(10));
    }

    #[test]
    fn from_rust_bumps_multiplicity_at_end_pc() {
        let rom = rom_with_indexed_insts(0x8000_0000, 3);
        let end_pc = 0x8000_0008; // paddr of inst with index=2
        let stats = CounterStats { inst_count: atomics_from(&[1, 1, 1]), end_pc, steps: 100 };
        let main_len = MainTrace::<()>::NUM_ROWS as u64;
        let expected_bump = main_len - 100 % main_len;

        let air =
            RomInstance::compute_witness_from_rust::<F>(&rom, &stats, vec![F::from_u64(0); 10]);

        assert_eq!(air.trace[0], F::from_u64(1));
        assert_eq!(air.trace[1], F::from_u64(1));
        assert_eq!(air.trace[2], F::from_u64(1 + expected_bump));
    }

    #[test]
    fn from_rust_leaves_trace_untouched_when_all_zero() {
        let rom = rom_with_indexed_insts(0x8000_0000, 3);
        let stats =
            CounterStats { inst_count: atomics_from(&[0, 0, 0]), end_pc: 0xDEAD_BEEF, steps: 0 };
        // Sentinel value to detect any unintended writes.
        let sentinel = F::from_u64(99);
        let buf = vec![sentinel; 10];

        let air = RomInstance::compute_witness_from_rust::<F>(&rom, &stats, buf);

        for i in 0..10 {
            assert_eq!(air.trace[i], sentinel);
        }
    }

    /// Build a ROM that satisfies `get_instruction(ROM_EXIT)` by populating
    /// `rom_entry_instructions` so that index `(ROM_EXIT - 0x1000) >> 2` (= 1) returns
    /// a `ZiskInst` whose `index` field is `exit_trace_index`.
    fn rom_with_exit(exit_trace_index: u64) -> ZiskRom {
        let mut rom = ZiskRom { min_program_pc: 0x8000_0000, ..Default::default() };
        let exit_slot = ((ROM_EXIT - 0x1000) >> 2) as usize;
        rom.rom_entry_instructions = vec![ZiskInst::default(); exit_slot + 1];
        rom.rom_entry_instructions[exit_slot].index = exit_trace_index;
        rom
    }

    #[test]
    fn from_asm_copies_histogram_and_patches_exit_row() {
        let exit_trace_index = 2_u64;
        let rom = rom_with_exit(exit_trace_index);
        // The histogram's value at `exit_trace_index` must be exactly 1 — the assembly
        // runner is expected to record the exit instruction as executed once.
        let asm_romh = AsmRHData::new(/* steps */ 50, vec![3, 0, 1]);
        let main_len = MainTrace::<()>::NUM_ROWS as u64;
        let expected_exit = 1 + main_len - 50 % main_len;

        let air =
            RomInstance::compute_witness_from_asm::<F>(&rom, &asm_romh, vec![F::from_u64(0); 10]);

        assert_eq!(air.trace[0], F::from_u64(3));
        assert_eq!(air.trace[1], F::from_u64(0)); // zero-multiplicity entries are skipped
        assert_eq!(air.trace[2], F::from_u64(expected_exit));
    }

    #[test]
    #[should_panic(expected = "exit instruction should have been executed once")]
    fn from_asm_panics_when_histogram_lacks_exit_record() {
        let rom = rom_with_exit(/* exit_trace_index */ 2);
        // Histogram does NOT mark index 2 as executed → soundness assert must fire.
        let asm_romh = AsmRHData::new(50, vec![3, 0, 0]);
        let _ =
            RomInstance::compute_witness_from_asm::<F>(&rom, &asm_romh, vec![F::from_u64(0); 10]);
    }

    #[test]
    fn rust_reset_zeroes_inst_count() {
        let inst_count = atomics_from(&[7, 0, 13, 42]);
        let inst =
            RomInstance::new_rust(Arc::new(ZiskRom::default()), dummy_ictx(), inst_count.clone());

        <RomInstance as Instance<F>>::reset(&inst);

        for slot in inst_count.iter() {
            assert_eq!(slot.load(Ordering::Relaxed), 0);
        }
    }

    #[test]
    fn asm_reset_leaves_mode_intact() {
        // In ASM mode, reset() is documented as a no-op (the histogram is source input,
        // not derived state). After reset the instance must still be in ASM mode so the
        // next compute_witness can consume the same rh_data.
        let rh_data = AsmRunnerRH::new(AsmRHData::new(50, vec![3, 0, 1]));
        let inst = RomInstance::new_asm(Arc::new(ZiskRom::default()), dummy_ictx(), rh_data);

        <RomInstance as Instance<F>>::reset(&inst);

        assert!(inst.skip_collector(), "still in ASM mode after reset");
    }

    #[test]
    fn build_inputs_collector_returns_none_for_asm_mode() {
        let inst =
            RomInstance::new_asm(Arc::new(ZiskRom::default()), dummy_ictx(), asm_runner_rh_empty());

        let collector = <RomInstance as Instance<F>>::build_inputs_collector(&inst, ChunkId(0));
        assert!(collector.is_none());
    }

    #[test]
    fn build_inputs_collector_returns_some_for_rust_mode() {
        let inst = RomInstance::new_rust(
            Arc::new(ZiskRom::default()),
            dummy_ictx(),
            atomics_from(&[0; 4]),
        );

        let collector = <RomInstance as Instance<F>>::build_inputs_collector(&inst, ChunkId(0));
        assert!(collector.is_some());
    }

    #[test]
    fn rom_collector_process_data_updates_backing_counter() {
        let inst_count = atomics_from(&[0; 4]);
        let mut collector = RomCollector::new(inst_count.clone());

        // ROM bus payload layout: [step, pc, index, end].
        collector.process_data(&ROM_BUS_ID, &[10, 0x8000_0000, 2, 0]);
        assert_eq!(inst_count[2].load(Ordering::Relaxed), 1);

        collector.process_data(&ROM_BUS_ID, &[11, 0x8000_0004, 2, 1]);
        assert_eq!(inst_count[2].load(Ordering::Relaxed), 2);
    }

    #[test]
    fn build_rom_collector_reflects_mode() {
        let asm =
            RomInstance::new_asm(Arc::new(ZiskRom::default()), dummy_ictx(), asm_runner_rh_empty());
        assert!(asm.build_rom_collector(ChunkId(0)).is_none(), "ASM mode returns no collector");

        let rust =
            RomInstance::new_rust(Arc::new(ZiskRom::default()), dummy_ictx(), atomics_from(&[0]));
        assert!(rust.build_rom_collector(ChunkId(0)).is_some(), "Rust mode yields a collector");
    }

    #[test]
    fn aggregate_stats_merges_collector_end_pc_and_steps() {
        let state = RustState::new(atomics_from(&[0; 4]));

        // CounterStats::+= only carries forward non-default `end_pc`/`steps`
        // (see `common/src/component/component_counter.rs`). Per-instruction counts
        // are accumulated through the shared `Arc<Vec<AtomicU64>>`, not the merge.
        let mut c = RomCollector::new(state.inst_count.clone());
        c.rom_counter.counter_stats.end_pc = 0xAAAA;
        c.rom_counter.counter_stats.steps = 100;

        let collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)> = vec![(0, Box::new(c))];

        let stats = state.aggregate_stats(collectors).expect("aggregation should succeed");
        assert_eq!(stats.end_pc, 0xAAAA);
        assert_eq!(stats.steps, 100);
    }

    #[test]
    fn aggregate_stats_rejects_non_rom_collector() {
        struct WrongCollector;
        impl BusDevice<u64> for WrongCollector {
            fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }

        let state = RustState::new(atomics_from(&[0]));
        let collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)> =
            vec![(0, Box::new(WrongCollector))];

        let err = state.aggregate_stats(collectors).expect_err("wrong collector type must fail");
        assert!(matches!(err, RomError::BadCollectorType), "got {err:?}");
    }
}
