use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use crate::{
    DeviceMetricsList, DummyCounter, NestedDeviceMetricsList, StaticSMBundle, MAX_NUM_STEPS,
};
use asm_runner::{
    write_input, AsmMTHeader, AsmRunnerMO, AsmRunnerMT, AsmRunnerRH, AsmServices, AsmSharedMemory,
    MinimalTraces, PreloadedMO, PreloadedMT, PreloadedRH, SharedMemoryWriter,
};
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use rayon::prelude::*;
use sm_rom::RomSM;
#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;
use zisk_common::{io::ZiskStdin, ChunkId, EmuTrace, ExecutorStatsHandle, ZiskExecutionResult};
use zisk_core::{ZiskRom, MAX_INPUT_SIZE};
use ziskemu::ZiskEmulator;

pub struct EmulatorAsm {
    /// ZisK ROM, a binary file containing the ZisK program to be executed.
    pub zisk_rom: Arc<ZiskRom>,

    /// World rank for distributed execution. Default to 0 for single-node execution.
    world_rank: i32,

    /// Local rank for distributed execution. Default to 0 for single-node execution.
    local_rank: i32,

    /// Optional baseline port to communicate with assembly microservices.
    base_port: Option<u16>,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    unlock_mapped_memory: bool,

    /// Chunk size for processing.
    chunk_size: u64,

    /// Optional ROM state machine, used for assembly ROM execution.
    rom_sm: Option<Arc<RomSM>>,

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    asm_shmem_mt: Arc<Mutex<PreloadedMT>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    asm_shmem_mo: Arc<Mutex<PreloadedMO>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    asm_shmem_rh: Arc<Mutex<Option<PreloadedRH>>>,

    /// Shared memory writers for each assembly service.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    shmem_input_writer: [Arc<Mutex<Option<SharedMemoryWriter>>>; AsmServices::SERVICES.len()],
}

impl EmulatorAsm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        zisk_rom: Arc<ZiskRom>,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        chunk_size: u64,
        rom_sm: Option<Arc<RomSM>>,
    ) -> Self {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mt = PreloadedMT::new(local_rank, base_port, unlock_mapped_memory)
            .expect("Failed to create PreloadedMT");
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mo = PreloadedMO::new(local_rank, base_port, unlock_mapped_memory)
            .expect("Failed to create PreloadedMO");

        Self {
            zisk_rom,
            world_rank,
            local_rank,
            base_port,
            unlock_mapped_memory,
            chunk_size,
            rom_sm,
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            asm_shmem_mt: Arc::new(Mutex::new(asm_shmem_mt)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            asm_shmem_mo: Arc::new(Mutex::new(asm_shmem_mo)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            asm_shmem_rh: Arc::new(Mutex::new(None)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            shmem_input_writer: std::array::from_fn(|_| Arc::new(Mutex::new(None))),
        }
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
    /// * `MinimalTraces` - The computed minimal traces.
    /// * `DeviceMetricsList` - Flat device metrics collected during execution.
    /// * `NestedDeviceMetricsList` - Hierarchical device metrics collected during execution.
    /// * `Option<JoinHandle<AsmRunnerMO>>` - Optional join handle for the memory-only ASM runner.
    /// * `ZiskExecutionResult` - The result of executing the ZisK ROM.
    #[allow(clippy::type_complexity)]
    pub fn execute<F: PrimeField64>(
        &self,
        stdin: &Mutex<ZiskStdin>,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        _caller_stats_id: u64,
    ) -> (
        MinimalTraces,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        ZiskExecutionResult,
    ) {
        #[cfg(feature = "stats")]
        let parent_stats_id = stats.next_id();
        #[cfg(feature = "stats")]
        stats.add_stat(
            _caller_stats_id,
            parent_stats_id,
            "EXECUTE_WITH_ASSEMBLY",
            0,
            ExecutorStatsEvent::Begin,
        );

        #[cfg(feature = "stats")]
        let stats_id = stats.next_id();
        #[cfg(feature = "stats")]
        stats.add_stat(parent_stats_id, stats_id, "ASM_WRITE_INPUT", 0, ExecutorStatsEvent::Begin);

        AsmServices::SERVICES.par_iter().enumerate().for_each(|(idx, service)| {
            let mut input_writer = self.shmem_input_writer[idx].lock().unwrap();
            input_writer.get_or_insert_with(|| self.create_shmem_writer(service));

            write_input(&mut stdin.lock().unwrap(), input_writer.as_ref().unwrap());
        });

        #[cfg(feature = "stats")]
        stats.add_stat(parent_stats_id, stats_id, "ASM_WRITE_INPUT", 0, ExecutorStatsEvent::End);

        let chunk_size = self.chunk_size;
        let (world_rank, local_rank, base_port) =
            (self.world_rank, self.local_rank, self.base_port);

        let _stats = stats.clone();

        // Run the assembly Memory Operations (MO) runner thread
        let handle_mo = std::thread::spawn({
            let asm_shmem_mo = self.asm_shmem_mo.clone();
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
        let has_rom_sm = pctx.dctx_is_first_partition();

        let _stats = stats.clone();

        let handle_rh = (has_rom_sm).then(|| {
            let asm_shmem_rh = self.asm_shmem_rh.clone();
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

        let (min_traces, main_count, secn_count) = self.run_mt_assembly(sm_bundle, stats);

        // Store execute steps
        let steps = if let MinimalTraces::AsmEmuTrace(asm_min_traces) = &min_traces {
            asm_min_traces.vec_chunks.iter().map(|trace| trace.steps).sum::<u64>()
        } else {
            panic!("Expected AsmEmuTrace, got something else");
        };

        let execution_result = ZiskExecutionResult::new(steps);

        // If the world rank is 0, wait for the ROM Histogram thread to finish and set the handler
        if has_rom_sm {
            self.rom_sm.as_ref().unwrap().set_asm_runner_handler(
                handle_rh.expect("Error during Assembly ROM Histogram thread execution"),
            );
        }

        #[cfg(feature = "stats")]
        stats.add_stat(0, parent_stats_id, "EXECUTE_WITH_ASSEMBLY", 0, ExecutorStatsEvent::End);

        (min_traces, main_count, secn_count, Some(handle_mo), execution_result)
    }

    fn create_shmem_writer(&self, service: &asm_runner::AsmService) -> SharedMemoryWriter {
        let port = if let Some(base_port) = self.base_port {
            AsmServices::port_for(service, base_port, self.local_rank)
        } else {
            AsmServices::default_port(service, self.local_rank)
        };

        let shmem_input_name =
            AsmSharedMemory::<AsmMTHeader>::shmem_input_name(port, *service, self.local_rank);

        tracing::debug!(
            "Initializing SharedMemoryWriter for service {:?} at '{}'",
            service,
            shmem_input_name
        );

        SharedMemoryWriter::new(
            &shmem_input_name,
            MAX_INPUT_SIZE as usize,
            self.unlock_mapped_memory,
        )
        .expect("Failed to create SharedMemoryWriter")
    }

    fn run_mt_assembly<F: PrimeField64>(
        &self,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
    ) -> (MinimalTraces, DeviceMetricsList, NestedDeviceMetricsList) {
        #[cfg(feature = "stats")]
        let parent_stats_id = stats.next_id();
        #[cfg(feature = "stats")]
        stats.add_stat(0, parent_stats_id, "RUN_MT_ASSEMBLY", 0, ExecutorStatsEvent::Begin);

        let results_mu: Mutex<Vec<(ChunkId, _)>> = Mutex::new(Vec::new());

        let emu_traces = rayon::in_place_scope(|scope| {
            let on_chunk = |idx: usize, emu_trace: std::sync::Arc<EmuTrace>| {
                let chunk_id = ChunkId(idx);
                let zisk_rom = &self.zisk_rom;
                let results_ref = &results_mu;
                scope.spawn(move |_| {
                    #[cfg(feature = "stats")]
                    let stats_id = stats.next_id();
                    #[cfg(feature = "stats")]
                    stats.add_stat(
                        parent_stats_id,
                        stats_id,
                        "MT_CHUNK_PLAYER",
                        0,
                        ExecutorStatsEvent::Begin,
                    );

                    let mut data_bus = sm_bundle.build_data_bus_counters();

                    ZiskEmulator::process_emu_trace::<F, _, _>(
                        zisk_rom,
                        &emu_trace,
                        &mut data_bus,
                        false,
                    );

                    data_bus.on_close();

                    #[cfg(feature = "stats")]
                    stats.add_stat(
                        parent_stats_id,
                        stats_id,
                        "MT_CHUNK_PLAYER",
                        0,
                        ExecutorStatsEvent::End,
                    );

                    results_ref.lock().unwrap().push((chunk_id, data_bus));
                });
            };

            AsmRunnerMT::run_and_count(
                &mut self.asm_shmem_mt.lock().unwrap(),
                MAX_NUM_STEPS,
                self.chunk_size,
                on_chunk,
                self.world_rank,
                self.local_rank,
                self.base_port,
                stats.clone(),
            )
            .expect("Error during ASM execution")
        });

        // Unwrap the Arc pointers now that all rayon tasks have completed
        let emu_traces = emu_traces
            .into_iter()
            .map(|arc| Arc::try_unwrap(arc).expect("Arc should have single owner after scope"))
            .collect();
        let asm_runner_mt = AsmRunnerMT::new(emu_traces);

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

        #[cfg(feature = "stats")]
        stats.add_stat(0, parent_stats_id, "RUN_MT_ASSEMBLY", 0, ExecutorStatsEvent::End);
        (MinimalTraces::AsmEmuTrace(asm_runner_mt), main_count, secn_count)
    }
}

impl<F: PrimeField64> crate::Emulator<F> for EmulatorAsm {
    fn execute(
        &self,
        stdin: &Mutex<ZiskStdin>,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        caller_stats_id: u64,
    ) -> (
        MinimalTraces,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        ZiskExecutionResult,
    ) {
        self.execute(stdin, pctx, sm_bundle, stats, caller_stats_id)
    }
}
