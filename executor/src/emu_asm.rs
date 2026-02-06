use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use crate::{
    DeviceMetricsList, DummyCounter, NestedDeviceMetricsList, StaticSMBundle, MAX_NUM_STEPS,
};
use asm_runner::{
    shmem_input_name, write_input, AsmRunnerMO, AsmRunnerMT, AsmRunnerRH, AsmService, AsmServices,
    MOOutputShmem, MTOutputShmem, RHOutputShmem, SharedMemoryWriter,
};
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use sm_rom::RomSM;
use zisk_common::{
    io::ZiskStdin, stats_begin, stats_end, ChunkId, EmuTrace, ExecutorStatsHandle, StatsScope,
    ZiskExecutionResult,
};
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
    asm_shmem_mt: Arc<Mutex<MTOutputShmem>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    asm_shmem_mo: Arc<Mutex<MOOutputShmem>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    asm_shmem_rh: Arc<Mutex<Option<RHOutputShmem>>>,

    /// Shared memory writers for each assembly service.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    shmem_input_writer: Arc<Mutex<Option<SharedMemoryWriter>>>,
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
        let asm_shmem_mt = MTOutputShmem::new(local_rank, base_port, unlock_mapped_memory)
            .expect("Failed to create PreloadedMT");
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mo = MOOutputShmem::new(local_rank, base_port, unlock_mapped_memory)
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
            shmem_input_writer: Arc::new(Mutex::new(None)),
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
    /// * `Vec<EmuTrace>` - The computed minimal traces.
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
        _caller_stats_scope: &StatsScope,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        ZiskExecutionResult,
    ) {
        stats_begin!(stats, _caller_stats_scope, _exec_scope, "EXECUTE_WITH_ASSEMBLY", 0);

        stats_begin!(stats, &_exec_scope, _write_scope, "ASM_WRITE_INPUT", 0);

        let mut input_writer = self.shmem_input_writer.lock().unwrap();
        input_writer.get_or_insert_with(|| self.create_shmem_writer(&AsmService::MO));

        write_input(&mut stdin.lock().unwrap(), input_writer.as_ref().unwrap());

        stats_end!(stats, &_write_scope);

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
        let steps = min_traces.iter().map(|trace| trace.steps).sum::<u64>();

        let execution_result = ZiskExecutionResult::new(steps);

        // If the world rank is 0, wait for the ROM Histogram thread to finish and set the handler
        if has_rom_sm {
            self.rom_sm.as_ref().unwrap().set_asm_runner_handler(
                handle_rh.expect("Error during Assembly ROM Histogram thread execution"),
            );
        }

        stats_end!(stats, &_exec_scope);

        (min_traces, main_count, secn_count, Some(handle_mo), execution_result)
    }

    fn create_shmem_writer(&self, service: &asm_runner::AsmService) -> SharedMemoryWriter {
        let port = AsmServices::port_base_for(self.base_port, self.local_rank);

        let shmem_input_name = shmem_input_name(port, self.local_rank);

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
    ) -> (Vec<EmuTrace>, DeviceMetricsList, NestedDeviceMetricsList) {
        stats_begin!(stats, 0, _mt_scope, "RUN_MT_ASSEMBLY", 0);

        let results_mu: Mutex<Vec<(ChunkId, _)>> = Mutex::new(Vec::new());

        // Capture the parent scope ID so it can be copied into the closure
        #[allow(unused_variables)]
        let mt_scope_id = _mt_scope.id();

        let emu_traces = rayon::in_place_scope(|scope| {
            let on_chunk = |idx: usize, emu_trace: std::sync::Arc<EmuTrace>| {
                let chunk_id = ChunkId(idx);
                let zisk_rom = &self.zisk_rom;
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
        stdin: &Mutex<ZiskStdin>,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        ZiskExecutionResult,
    ) {
        self.execute(stdin, pctx, sm_bundle, stats, caller_stats_scope)
    }
}
