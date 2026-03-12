use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use crate::AsmResources;
use crate::{
    DeviceMetricsList, DummyCounter, NestedDeviceMetricsList, StaticSMBundle, MAX_NUM_STEPS,
};
use asm_runner::{AsmRunnerMO, AsmRunnerMT, AsmRunnerRH};
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use sm_rom::RomSM;
use zisk_common::{
    io::ZiskStdin, stats_begin, stats_end, AsmExecutionInfo, ChunkId, EmuTrace,
    ExecutorStatsHandle, StatsScope,
};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use anyhow::Result;

pub struct EmulatorAsm {
    /// World rank for distributed execution. Default to 0 for single-node execution.
    world_rank: i32,

    /// Local rank for distributed execution. Default to 0 for single-node execution.
    local_rank: i32,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    unlock_mapped_memory: bool,

    /// Chunk size for processing.
    chunk_size: u64,

    /// Optional ROM state machine, used for assembly ROM execution.
    rom_sm: Option<Arc<RomSM>>,

    /// Assembly resources including shared memory and hints stream.
    asm_resources: Mutex<Option<AsmResources>>,

    asm_execution_info: Mutex<Option<AsmExecutionInfo>>,
}

impl EmulatorAsm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_rank: i32,
        local_rank: i32,
        unlock_mapped_memory: bool,
        chunk_size: u64,
        rom_sm: Option<Arc<RomSM>>,
        _verbose_mode: proofman_common::VerboseMode,
    ) -> Self {
        Self {
            world_rank,
            local_rank,
            unlock_mapped_memory,
            chunk_size,
            rom_sm,
            asm_resources: Mutex::new(None),
            asm_execution_info: Mutex::new(None),
        }
    }

    pub fn get_chunk_size(&self) -> u64 {
        self.chunk_size
    }

    pub fn get_asm_execution_info(&self) -> Option<AsmExecutionInfo> {
        self.asm_execution_info.lock().unwrap().clone()
    }

    pub fn set_asm_resources(&self, asm_resources: AsmResources) {
        *self.asm_resources.lock().unwrap() = Some(asm_resources);
    }

    pub fn reset_hints_stream(&self) {
        self.asm_resources.lock().unwrap().as_ref().unwrap().reset();
    }

    pub fn set_rh_data(&self, rh_data: AsmRunnerRH) {
        self.rom_sm.as_ref().unwrap().set_rh_data(rh_data);
    }

    /// Computes minimal traces by processing the ZisK ROM with given public inputs.
    ///
    /// # Arguments
    /// * `stdin` - Shared mutable access to the ZiskStdin providing public inputs.
    /// * `pctx` - Proof context used during execution.
    /// * `sm_bundle` - Static shared-memory bundle used by the executor.
    /// * `stats` - Handle for collecting executor statistics.
    /// * `_caller_stats_id` - Identifier used to attribute collected statistics to the caller.
    ///
    /// # Returns
    /// A tuple containing:
    /// * `Vec<EmuTrace>` - The computed minimal traces.
    /// * `DeviceMetricsList` - Flat device metrics collected during execution.
    /// * `NestedDeviceMetricsList` - Hierarchical device metrics collected during execution.
    /// * `Option<JoinHandle<AsmRunnerMO>>` - Optional join handle for the memory-only ASM runner.
    /// * `u64` - Total number of steps.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pub fn execute<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &Mutex<ZiskStdin>,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        _caller_stats_scope: &StatsScope,
    ) -> Result<(
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        Option<JoinHandle<AsmRunnerRH>>,
        u64,
    )> {
        let asm_resources_guard = self.asm_resources.lock().unwrap();
        let asm_resources = asm_resources_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AsmResources not initialized"))?;

        let has_hints_stream = asm_resources.is_hints_stream_initialized();
        if use_hints && has_hints_stream {
            asm_resources.start_stream()?;
        }

        stats_begin!(stats, _caller_stats_scope, _exec_scope, "EXECUTE_WITH_ASSEMBLY", 0);

        stats_begin!(stats, &_exec_scope, _write_scope, "ASM_WRITE_INPUT", 0);

        let config = asm_resources.config();

        asm_resources.write_input(&stdin.lock().unwrap())?;

        stats_end!(stats, &_write_scope);

        let chunk_size = self.chunk_size;
        let (world_rank, local_rank) = (self.world_rank, self.local_rank);

        let _stats = stats.clone();

        // Run the assembly Memory Operations (MO) runner thread
        let handle_mo = std::thread::spawn({
            let asm_shmem_mo = asm_resources.mo_shmem_reader.clone();
            let base_port = config.base_port;
            move || {
                AsmRunnerMO::run(
                    &mut asm_shmem_mo.lock().unwrap(),
                    MAX_NUM_STEPS,
                    chunk_size,
                    world_rank,
                    local_rank,
                    base_port,
                    _stats,
                )
                .expect("Error during Assembly Memory Operations execution")
            }
        });

        // Run the ROM histogram only on partition 0 as it is always computed by this partition
        let has_rom_sm = pctx.dctx_is_first_process();

        let _stats = stats.clone();

        let handle_rh = (has_rom_sm).then(|| {
            let asm_shmem_rh = asm_resources.rh_shmem_reader.clone();
            let base_port = config.base_port;
            let unlock_mapped_memory = self.unlock_mapped_memory;
            std::thread::spawn(move || {
                AsmRunnerRH::run(
                    &mut asm_shmem_rh.lock().unwrap(),
                    MAX_NUM_STEPS,
                    world_rank,
                    local_rank,
                    base_port,
                    unlock_mapped_memory,
                    _stats,
                )
                .expect("Error during ROM Histogram execution")
            })
        });
        drop(asm_resources_guard);

        let (min_traces, main_count, secn_count) = self.run_mt_assembly(zisk_rom, sm_bundle, stats);
        // Store execute steps
        let steps = min_traces.iter().map(|trace| trace.steps).sum::<u64>();

        stats_end!(stats, &_exec_scope);

        Ok((min_traces, main_count, secn_count, Some(handle_mo), handle_rh, steps))
    }

    fn run_mt_assembly<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
    ) -> (Vec<EmuTrace>, DeviceMetricsList, NestedDeviceMetricsList) {
        stats_begin!(stats, 0, _mt_scope, "RUN_MT_ASSEMBLY", 0);

        let results_mu: Mutex<Vec<(ChunkId, _)>> = Mutex::new(Vec::new());

        // Capture the parent scope ID so it can be copied into the closure
        #[allow(unused_variables)]
        let mt_scope_id = _mt_scope.id();

        let (emu_traces, asm_execution_info) = rayon::in_place_scope(|scope| {
            let on_chunk = |idx: usize, emu_trace: std::sync::Arc<EmuTrace>| {
                let chunk_id = ChunkId(idx);
                let results_ref = &results_mu;
                scope.spawn(move |_| {
                    stats_begin!(stats, mt_scope_id, _chunk_scope, "MT_CHUNK_PLAYER", 0);

                    let mut data_bus = sm_bundle.build_data_bus_counters();

                    ZiskEmulator::process_emu_trace::<F, _, _>(
                        zisk_rom,
                        &emu_trace,
                        &mut data_bus,
                        false,
                    );

                    data_bus.on_close();

                    stats_end!(stats, &_chunk_scope);

                    results_ref.lock().unwrap().push((chunk_id, data_bus));
                });
            };

            let asm_resources_guard = self.asm_resources.lock().unwrap();
            let asm_resources = asm_resources_guard.as_ref().expect("AsmResources not initialized");
            let result = AsmRunnerMT::run_and_count(
                &mut asm_resources.mt_shmem_reader.lock().unwrap(),
                MAX_NUM_STEPS,
                self.chunk_size,
                on_chunk,
                self.world_rank,
                self.local_rank,
                asm_resources.config().base_port,
                stats.clone(),
            )
            .expect("Error during ASM execution");
            drop(asm_resources_guard);
            result
        });

        self.asm_execution_info.lock().unwrap().replace(asm_execution_info);

        // Unwrap the Arc pointers now that all rayon tasks have completed
        let emu_traces = emu_traces
            .into_iter()
            .map(|arc| Arc::try_unwrap(arc).expect("Arc should have single owner after scope"))
            .collect();

        let mut data_buses = results_mu.into_inner().unwrap();

        data_buses.sort_by_key(|(chunk_id, _)| chunk_id.0);

        let mut main_count = Vec::with_capacity(data_buses.len());
        let mut secn_count = HashMap::new();

        for (chunk_id, data_bus) in data_buses {
            let databus_counters = data_bus.into_devices(false);

            for (idx, counter) in databus_counters.into_iter() {
                match idx {
                    None => {
                        main_count.push((chunk_id, counter.unwrap_or(Box::new(DummyCounter {}))));
                    }
                    Some(idx) => {
                        secn_count
                            .entry(idx)
                            .or_insert_with(Vec::new)
                            .push((chunk_id, counter.unwrap()));
                    }
                }
            }
        }

        stats_end!(stats, &_mt_scope);
        (emu_traces, main_count, secn_count)
    }
}

impl<F: PrimeField64> crate::Emulator<F> for EmulatorAsm {
    fn execute(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &Mutex<ZiskStdin>,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> Result<(
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        Option<JoinHandle<AsmRunnerRH>>,
        u64,
    )> {
        self.execute(zisk_rom, stdin, pctx, sm_bundle, use_hints, stats, caller_stats_scope)
    }
}
