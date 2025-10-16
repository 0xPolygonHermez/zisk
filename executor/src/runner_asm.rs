use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use asm_runner::{
    write_input, AsmMTHeader, AsmRunnerMO, AsmRunnerMT, AsmRunnerRH, AsmServices, AsmSharedMemory,
    MinimalTraces, PreloadedMO, PreloadedMT, PreloadedRH, Task, TaskFactory,
};
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;

use rayon::prelude::*;
#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;
use zisk_common::{BusDeviceMetrics, ChunkId, EmuTrace, ExecutorStatsHandle, PayloadType};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use crate::{
    DeviceMetricsList, DummyCounter, ExecutionResult, ExecutionResultEnum, ExecutorRunner,
    NestedDeviceMetricsList, StaticSMBundle,
};

pub struct AssemblyRunner<F> {
    world_rank: i32,
    local_rank: i32,
    base_port: Option<u16>,
    asm_shmem_mt: Arc<Mutex<Option<PreloadedMT>>>,
    asm_shmem_mo: Arc<Mutex<Option<PreloadedMO>>>,
    asm_shmem_rh: Arc<Mutex<Option<PreloadedRH>>>,
    unlock_mapped_memory: bool,
    _phantom: std::marker::PhantomData<F>,
    handle_rh: Option<JoinHandle<AsmRunnerRH>>,
    handle_mo: Option<JoinHandle<AsmRunnerMO>>,
}

impl<F: PrimeField64> AssemblyRunner<F> {
    pub fn new(
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        asm_path: Option<PathBuf>,
        unlock_mapped_memory: bool,
    ) -> Self {
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        let (asm_shmem_mt, asm_shmem_mo) = (None, None);

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let (asm_shmem_mt, asm_shmem_mo) = if asm_path.is_some() {
            let mt = PreloadedMT::new(local_rank, base_port, unlock_mapped_memory)
                .expect("Failed to create PreloadedMT");
            let mo = PreloadedMO::new(local_rank, base_port, unlock_mapped_memory)
                .expect("Failed to create PreloadedMO");
            (Some(mt), Some(mo))
        } else {
            (None, None)
        };

        Self {
            world_rank,
            local_rank,
            base_port,
            asm_shmem_mt: Arc::new(Mutex::new(asm_shmem_mt)),
            asm_shmem_mo: Arc::new(Mutex::new(asm_shmem_mo)),
            asm_shmem_rh: Arc::new(Mutex::new(None)),
            unlock_mapped_memory,
            _phantom: std::marker::PhantomData,
            handle_rh: None,
            handle_mo: None,
        }
    }

    /// Computes minimal traces by processing the ZisK ROM with given public inputs.
    ///
    /// # Arguments
    /// * `input_data` - Input data for the ROM execution.
    /// * `num_threads` - Number of threads to use for parallel execution.
    ///
    /// # Returns
    /// A vector of `EmuTrace` instances representing minimal traces.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        pctx: &ProofCtx<F>,
        zisk_rom: &ZiskRom,
        input_data_path: Option<PathBuf>,
        chunk_size: u64,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        #[cfg(feature = "stats")] _caller_stats_id: u64,
    ) -> (
        MinimalTraces,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        Option<JoinHandle<AsmRunnerRH>>,
        u64,
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

        if let Some(input_path) = input_data_path.as_ref() {
            AsmServices::SERVICES.par_iter().for_each(|service| {
                #[cfg(feature = "stats")]
                let stats_id = stats.next_id();
                #[cfg(feature = "stats")]
                stats.add_stat(
                    parent_stats_id,
                    stats_id,
                    "ASM_WRITE_INPUT",
                    0,
                    ExecutorStatsEvent::Begin,
                );

                let port = if let Some(base_port) = self.base_port {
                    AsmServices::port_for(service, base_port, self.local_rank)
                } else {
                    AsmServices::default_port(service, self.local_rank)
                };

                let shmem_input_name = AsmSharedMemory::<AsmMTHeader>::shmem_input_name(
                    port,
                    *service,
                    self.local_rank,
                );
                write_input(input_path, &shmem_input_name, self.unlock_mapped_memory);

                // Add to executor stats
                #[cfg(feature = "stats")]
                stats.add_stat(
                    parent_stats_id,
                    stats_id,
                    "ASM_WRITE_INPUT",
                    0,
                    ExecutorStatsEvent::End,
                );
            });
        }

        let (world_rank, local_rank, base_port) =
            (self.world_rank, self.local_rank, self.base_port);

        let _stats = stats.clone();

        // Run the assembly Memory Operations (MO) runner thread
        let asm_shmem_mo = self.asm_shmem_mo.clone();
        let handle_mo = std::thread::spawn({
            move || {
                AsmRunnerMO::run(
                    asm_shmem_mo.lock().unwrap().as_mut().unwrap(),
                    crate::MAX_NUM_STEPS,
                    chunk_size,
                    world_rank,
                    local_rank,
                    base_port,
                    _stats,
                )
                .expect("Error during Assembly Memory Operations execution")
            }
        });

        let _stats = stats.clone();

        // Run the ROM histogram only on partition 0 as it is always computed by this partition
        let has_rom_sm = pctx.dctx_is_first_partition();
        let handle_rh = (has_rom_sm).then(|| {
            let asm_shmem_rh = self.asm_shmem_rh.clone();
            let unlock_mapped_memory = self.unlock_mapped_memory;
            std::thread::spawn(move || {
                AsmRunnerRH::run(
                    &mut asm_shmem_rh.lock().unwrap(),
                    crate::MAX_NUM_STEPS,
                    world_rank,
                    local_rank,
                    base_port,
                    unlock_mapped_memory,
                    _stats,
                )
                .expect("Error during ROM Histogram execution")
            })
        });

        let (min_traces, main_count, secn_count) = Self::run_mt_assembly(
            zisk_rom,
            chunk_size,
            world_rank,
            local_rank,
            base_port,
            stats,
            self.asm_shmem_mt.clone(),
            sm_bundle,
        );

        // Store execute steps
        let steps = if let MinimalTraces::AsmEmuTrace(asm_min_traces) = &min_traces {
            asm_min_traces.vec_chunks.iter().map(|trace| trace.steps).sum::<u64>()
        } else {
            panic!("Expected AsmEmuTrace, got something else");
        };

        #[cfg(feature = "stats")]
        stats.add_stat(0, parent_stats_id, "EXECUTE_WITH_ASSEMBLY", 0, ExecutorStatsEvent::End);

        (min_traces, main_count, secn_count, Some(handle_mo), handle_rh, steps)
    }

    #[allow(clippy::too_many_arguments)]
    fn run_mt_assembly(
        zisk_rom: &ZiskRom,
        chunk_size: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        stats: &ExecutorStatsHandle,
        asm_shmem_mt: Arc<Mutex<Option<PreloadedMT>>>,
        sm_bundle: &StaticSMBundle<F>,
    ) -> (MinimalTraces, DeviceMetricsList, NestedDeviceMetricsList) {
        #[cfg(feature = "stats")]
        let parent_stats_id = stats.next_id();
        #[cfg(feature = "stats")]
        stats.add_stat(0, parent_stats_id, "RUN_MT_ASSEMBLY", 0, ExecutorStatsEvent::Begin);

        struct CounterTask<F, DB>
        where
            DB: DataBusTrait<PayloadType, Box<dyn BusDeviceMetrics>>,
        {
            chunk_id: ChunkId,
            emu_trace: Arc<EmuTrace>,
            data_bus: DB,
            zisk_rom: Arc<ZiskRom>,
            _phantom: std::marker::PhantomData<F>,
            _stats: ExecutorStatsHandle,
            _parent_stats_id: u64,
        }

        impl<F, DB> Task<'static> for CounterTask<F, DB>
        where
            F: PrimeField64,
            DB: DataBusTrait<PayloadType, Box<dyn BusDeviceMetrics>> + Send + Sync + 'static,
        {
            type Output = (ChunkId, DB);

            fn execute(mut self) -> Self::Output {
                #[cfg(feature = "stats")]
                let stats_id = self._stats.next_id();
                #[cfg(feature = "stats")]
                self._stats.add_stat(
                    self._parent_stats_id,
                    stats_id,
                    "MT_CHUNK_PLAYER",
                    0,
                    ExecutorStatsEvent::Begin,
                );

                ZiskEmulator::process_emu_trace::<F, _, _>(
                    &self.zisk_rom,
                    &self.emu_trace,
                    &mut self.data_bus,
                    false,
                );

                self.data_bus.on_close();

                // Add to executor stats
                #[cfg(feature = "stats")]
                self._stats.add_stat(
                    self._parent_stats_id,
                    stats_id,
                    "MT_CHUNK_PLAYER",
                    0,
                    ExecutorStatsEvent::End,
                );

                (self.chunk_id, self.data_bus)
            }
        }

        let task_factory: TaskFactory<_> = {
            let zisk_rom_arc = Arc::new(zisk_rom.clone());
            Box::new(move |chunk_id: ChunkId, emu_trace: Arc<EmuTrace>| {
                let data_bus = sm_bundle.build_data_bus_counters();
                CounterTask {
                    chunk_id,
                    emu_trace,
                    data_bus,
                    zisk_rom: Arc::clone(&zisk_rom_arc),
                    _phantom: std::marker::PhantomData::<F>,
                    _stats: stats.clone(),
                    #[cfg(feature = "stats")]
                    _parent_stats_id: parent_stats_id,
                    #[cfg(not(feature = "stats"))]
                    _parent_stats_id: 0,
                }
            })
        };

        let (asm_runner_mt, mut data_buses) = AsmRunnerMT::run_and_count(
            asm_shmem_mt.lock().unwrap().as_mut().unwrap(),
            crate::MAX_NUM_STEPS,
            chunk_size,
            task_factory,
            world_rank,
            local_rank,
            base_port,
            stats.clone(),
        )
        .expect("Error during ASM execution");

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

        (MinimalTraces::AsmEmuTrace(asm_runner_mt), main_count, secn_count)
    }
}

impl<F: PrimeField64> ExecutorRunner<F> for AssemblyRunner<F> {
    fn run(
        &mut self,
        pctx: &ProofCtx<F>,
        zisk_rom: &ZiskRom,
        input_data_path: Option<PathBuf>,
        chunk_size: u64,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        #[cfg(feature = "stats")] _caller_stats_id: u64,
    ) -> ExecutionResultEnum {
        let (minimal_traces, main, secn, handle_mo, handle_rh, steps) = self.run(
            pctx,
            zisk_rom,
            input_data_path,
            chunk_size,
            sm_bundle,
            stats,
            #[cfg(feature = "stats")]
            _caller_stats_id,
        );

        self.handle_mo = handle_mo;
        self.handle_rh = handle_rh;

        ExecutionResultEnum::PartialResult(ExecutionResult { minimal_traces, main, secn, steps })
    }

    fn finalize(&mut self) -> Option<(JoinHandle<AsmRunnerRH>, JoinHandle<AsmRunnerMO>)> {
        let asm_runner_mo = self.handle_mo.take()?;
        let asm_runner_rh = self.handle_rh.take()?;

        Some((asm_runner_rh, asm_runner_mo))
    }
}
