//! The `ZiskExecutor` module serves as the core orchestrator for executing the ZisK ROM program
//! and generating witness computations. It manages the execution of the state machines, from initial
//! planning to witness computation, ensuring efficient parallel processing and resource
//! utilization.
//!
//! This module handles both main and secondary state machines, integrating complex tasks such as
//! planning, configuration, and witness generation into a streamlined process.
//!
//! ## Executor Workflow
//! The execution is divided into distinct, sequential phases:
//!
//! 1. **Minimal Traces**: Rapidly process the ROM to collect minimal traces with minimal overhead.
//! 2. **Counting**: Creates the metrics required for the secondary state machine instances.
//! 3. **Planning**: Strategically plan the execution of instances to optimize resource usage.
//! 4. **Instance Creation**: Creates the AIR instances for the main and secondary state machines.
//! 5. **Witness Computation**: Compute the witnesses for all AIR instances, leveraging parallelism
//!    for efficiency.
//!
//! By structuring these phases, the `ZiskExecutor` ensures high-performance execution while
//! maintaining clarity and modularity in the computation process.

use asm_runner::{
    write_input, AsmMTHeader, AsmRunnerMO, AsmRunnerMT, AsmRunnerRH, AsmServices, AsmSharedMemory,
    MinimalTraces, PreloadedMO, PreloadedMT, PreloadedRH, Task, TaskFactory,
};
use crossbeam::queue::SegQueue;
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{create_pool, BufferPool, ProofCtx, SetupCtx};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use rayon::prelude::*;
use rom_setup::gen_elf_hash;
use sm_rom::RomSM;
use std::sync::atomic::{AtomicUsize, Ordering};
use witness::WitnessComponent;

use crate::DummyCounter;
use data_bus::DataBusTrait;
use sm_main::{MainInstance, MainPlanner, MainSM};
use zisk_common::{
    BusDevice, BusDeviceMetrics, CheckPoint, ExecutorStats, Instance, InstanceCtx, InstanceType,
    Plan,
};
use zisk_common::{ChunkId, PayloadType};
use zisk_pil::{RomRomTrace, ZiskPublicValues, MAIN_AIR_IDS, ROM_AIR_IDS, MEM_AIR_IDS, ROM_DATA_AIR_IDS, INPUT_DATA_AIR_IDS, ZISK_AIRGROUP_ID};

use std::thread::JoinHandle;
use std::time::Instant;
use std::{
    collections::HashMap,
    fmt::Debug,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};
#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;

use crossbeam::atomic::AtomicCell;

use zisk_common::EmuTrace;
use zisk_core::ZiskRom;
use ziskemu::{EmuOptions, ZiskEmulator};

use crate::SMBundle;

type DeviceMetricsByChunk = (ChunkId, Box<dyn BusDeviceMetrics>); // (chunk_id, metrics)
type DeviceMetricsList = Vec<DeviceMetricsByChunk>;
pub type NestedDeviceMetricsList = Vec<DeviceMetricsList>;

#[derive(Debug, Default, Clone)]
pub struct ZiskExecutionResult {
    pub executed_steps: u64,
}

#[allow(dead_code)]
enum MinimalTraceExecutionMode {
    Emulator,
    AsmWithCounter,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub airgroup_id: usize,
    pub air_id: usize,
    /// Collect start time
    pub collect_start_time: Instant,
    /// Collect duration in microseconds
    pub collect_duration: u64,
    /// Witness start time
    pub witness_start_time: Instant,
    /// Witness duration in microseconds
    pub witness_duration: u64,
    /// Number of chunks
    pub num_chunks: usize,
}

/// The `ZiskExecutor` struct orchestrates the execution of the ZisK ROM program, managing state
/// machines, planning, and witness computation.
pub struct ZiskExecutor<F: PrimeField64, BD: SMBundle<F>> {
    /// ZisK ROM, a binary file containing the ZisK program to be executed.
    pub zisk_rom: Arc<ZiskRom>,

    /// Path to the ZisK ROM file.
    pub rom_path: PathBuf,

    /// Path to the assembly minimal trace binary file, if applicable.
    pub asm_runner_path: Option<PathBuf>,

    /// Path to the assembly ROM binary file, if applicable.
    pub asm_rom_path: Option<PathBuf>,

    /// Planning information for main state machines.
    pub min_traces: Arc<RwLock<MinimalTraces>>,

    /// Planning information for main state machines.
    pub main_planning: RwLock<Vec<Plan>>,

    /// Planning information for secondary state machines.
    pub secn_planning: RwLock<Vec<Vec<Plan>>>,

    /// Main state machine instances, indexed by their global ID.
    pub main_instances: RwLock<HashMap<usize, MainInstance<F>>>,

    /// Secondary state machine instances, indexed by their global ID.
    pub secn_instances: RwLock<HashMap<usize, Box<dyn Instance<F>>>>,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,

    /// Execution result, including the number of executed steps.
    execution_result: Mutex<ZiskExecutionResult>,

    /// State machine bundle, containing the state machines and their configurations.
    sm_bundle: BD,

    /// Optional ROM state machine, used for assembly ROM execution.
    rom_sm: Option<Arc<RomSM>>,

    /// Collectors by instance, storing statistics and collectors for each instance.
    #[allow(clippy::type_complexity)]
    collectors_by_instance: Arc<RwLock<HashMap<usize, Vec<(usize, Box<dyn BusDevice<u64>>)>>>>,

    /// Statistics collected during the execution, including time taken for collection and witness computation.
    stats: Arc<Mutex<ExecutorStats>>,

    witness_stats: Arc<Mutex<HashMap<usize, Stats>>>,

    chunk_size: u64,

    /// World rank for distributed execution. Default to 0 for single-node execution.
    world_rank: i32,

    /// Local rank for distributed execution. Default to 0 for single-node execution.
    local_rank: i32,

    /// Optional baseline port to communicate with assembly microservices.
    base_port: Option<u16>,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    unlock_mapped_memory: bool,

    asm_shmem_mt: Arc<Mutex<Option<PreloadedMT>>>,
    asm_shmem_mo: Arc<Mutex<Option<PreloadedMO>>>,
    asm_shmem_rh: Arc<Mutex<Option<PreloadedRH>>>,
}

impl<F: PrimeField64, BD: SMBundle<F>> ZiskExecutor<F, BD> {
    /// The number of threads to use for parallel processing when computing minimal traces.
    const NUM_THREADS: usize = 16;

    /// The maximum number of steps to execute in the emulator or assembly runner.
    const MAX_NUM_STEPS: u64 = 1 << 32;

    /// Creates a new instance of the `ZiskExecutor`.
    ///
    /// # Arguments
    /// * `zisk_rom` - An `Arc`-wrapped ZisK ROM instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rom_path: PathBuf,
        asm_path: Option<PathBuf>,
        asm_rom_path: Option<PathBuf>,
        zisk_rom: Arc<ZiskRom>,
        std: Arc<Std<F>>,
        sm_bundle: BD,
        rom_sm: Option<Arc<RomSM>>,
        chunk_size: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
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
            rom_path,
            asm_runner_path: asm_path,
            asm_rom_path,
            zisk_rom,
            min_traces: Arc::new(RwLock::new(MinimalTraces::None)),
            main_planning: RwLock::new(Vec::new()),
            secn_planning: RwLock::new(Vec::new()),
            main_instances: RwLock::new(HashMap::new()),
            secn_instances: RwLock::new(HashMap::new()),
            collectors_by_instance: Arc::new(RwLock::new(HashMap::new())),
            std,
            execution_result: Mutex::new(ZiskExecutionResult::default()),
            sm_bundle,
            rom_sm,
            stats: Arc::new(Mutex::new(ExecutorStats::new())),
            witness_stats: Arc::new(Mutex::new(HashMap::new())),
            chunk_size,
            world_rank,
            local_rank,
            base_port,
            unlock_mapped_memory,
            asm_shmem_mt: Arc::new(Mutex::new(asm_shmem_mt)),
            asm_shmem_mo: Arc::new(Mutex::new(asm_shmem_mo)),
            asm_shmem_rh: Arc::new(Mutex::new(None)),
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn get_execution_result(
        &self,
    ) -> (ZiskExecutionResult, Arc<Mutex<ExecutorStats>>, Arc<Mutex<HashMap<usize, Stats>>>) {
        (
            self.execution_result.lock().unwrap().clone(),
            self.stats.clone(),
            self.witness_stats.clone(),
        )
    }

    pub fn store_stats(&self) {
        self.stats.lock().unwrap().store_stats();
    }

    /// Computes minimal traces by processing the ZisK ROM with given public inputs.
    ///
    /// # Arguments
    /// * `input_data` - Input data for the ROM execution.
    /// * `num_threads` - Number of threads to use for parallel execution.
    ///
    /// # Returns
    /// A vector of `EmuTrace` instances representing minimal traces.
    fn execute_with_emulator(&self, input_data_path: Option<PathBuf>) -> MinimalTraces {
        let min_traces = self.run_emulator(Self::NUM_THREADS, input_data_path);

        // Store execute steps
        let steps = if let MinimalTraces::EmuTrace(min_traces) = &min_traces {
            min_traces.iter().map(|trace| trace.steps).sum::<u64>()
        } else {
            panic!("Expected EmuTrace, got something else");
        };

        self.execution_result.lock().unwrap().executed_steps = steps;

        min_traces
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
    fn execute_with_assembly(
        &self,
        input_data_path: Option<PathBuf>,
        _caller_stats_id: u64,
    ) -> (MinimalTraces, DeviceMetricsList, NestedDeviceMetricsList, Option<JoinHandle<AsmRunnerMO>>)
    {
        #[cfg(feature = "stats")]
        let parent_stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            _caller_stats_id,
            parent_stats_id,
            "EXECUTE_WITH_ASSEMBLY",
            0,
            ExecutorStatsEvent::Begin,
        );

        if let Some(input_path) = input_data_path.as_ref() {
            AsmServices::SERVICES.par_iter().for_each(|service| {
                #[cfg(feature = "stats")]
                let stats_id = self.stats.lock().unwrap().get_id();
                #[cfg(feature = "stats")]
                self.stats.lock().unwrap().add_stat(
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
                self.stats.lock().unwrap().add_stat(
                    parent_stats_id,
                    stats_id,
                    "ASM_WRITE_INPUT",
                    0,
                    ExecutorStatsEvent::End,
                );
            });
        }

        let chunk_size = self.chunk_size;
        let (world_rank, local_rank, base_port) =
            (self.world_rank, self.local_rank, self.base_port);

        let stats = Arc::clone(&self.stats);

        // Run the assembly Memory Operations (MO) runner thread
        let handle_mo = std::thread::spawn({
            let asm_shmem_mo = self.asm_shmem_mo.clone();
            move || {
                AsmRunnerMO::run(
                    asm_shmem_mo.lock().unwrap().as_mut().unwrap(),
                    Self::MAX_NUM_STEPS,
                    chunk_size,
                    world_rank,
                    local_rank,
                    base_port,
                    stats,
                )
                .expect("Error during Assembly Memory Operations execution")
            }
        });

        let stats = Arc::clone(&self.stats);

        // Run the assembly ROM Histogram runner with the provided input data path only if the world rank is 0
        let handle_rh = (self.world_rank == 0).then(|| {
            let asm_shmem_rh = self.asm_shmem_rh.clone();
            let unlock_mapped_memory = self.unlock_mapped_memory;
            std::thread::spawn(move || {
                AsmRunnerRH::run(
                    &mut asm_shmem_rh.lock().unwrap(),
                    Self::MAX_NUM_STEPS,
                    world_rank,
                    local_rank,
                    base_port,
                    unlock_mapped_memory,
                    stats,
                )
                .expect("Error during ROM Histogram execution")
            })
        });

        let (min_traces, main_count, secn_count) = self.run_mt_assembly();

        // Store execute steps
        let steps = if let MinimalTraces::AsmEmuTrace(asm_min_traces) = &min_traces {
            asm_min_traces.vec_chunks.iter().map(|trace| trace.steps).sum::<u64>()
        } else {
            panic!("Expected AsmEmuTrace, got something else");
        };

        self.execution_result.lock().unwrap().executed_steps = steps;

        // If the world rank is 0, wait for the ROM Histogram thread to finish and set the handler
        if self.world_rank == 0 {
            self.rom_sm.as_ref().unwrap().set_asm_runner_handler(
                handle_rh.expect("Error during Assembly ROM Histogram thread execution"),
            );
        }

        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "EXECUTE_WITH_ASSEMBLY",
            0,
            ExecutorStatsEvent::End,
        );

        (min_traces, main_count, secn_count, Some(handle_mo))
    }

    fn run_mt_assembly(&self) -> (MinimalTraces, DeviceMetricsList, NestedDeviceMetricsList) {
        #[cfg(feature = "stats")]
        let parent_stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "RUN_MT_ASSEMBLY",
            0,
            ExecutorStatsEvent::Begin,
        );

        struct CounterTask<F, DB>
        where
            DB: DataBusTrait<PayloadType, Box<dyn BusDeviceMetrics>>,
        {
            chunk_id: ChunkId,
            emu_trace: Arc<EmuTrace>,
            data_bus: DB,
            zisk_rom: Arc<ZiskRom>,
            chunk_size: u64,
            _phantom: std::marker::PhantomData<F>,
            _stats: Arc<Mutex<ExecutorStats>>,
            _parent_stats_id: u64,
        }

        impl<F, DB> Task for CounterTask<F, DB>
        where
            F: PrimeField64,
            DB: DataBusTrait<PayloadType, Box<dyn BusDeviceMetrics>> + Send + Sync + 'static,
        {
            type Output = (ChunkId, DB);

            fn execute(mut self) -> Self::Output {
                #[cfg(feature = "stats")]
                let stats_id = self._stats.lock().unwrap().get_id();
                #[cfg(feature = "stats")]
                self._stats.lock().unwrap().add_stat(
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
                    self.chunk_size,
                    false,
                );

                self.data_bus.on_close();

                // Add to executor stats
                #[cfg(feature = "stats")]
                self._stats.lock().unwrap().add_stat(
                    self._parent_stats_id,
                    stats_id,
                    "MT_CHUNK_PLAYER",
                    0,
                    ExecutorStatsEvent::End,
                );

                (self.chunk_id, self.data_bus)
            }
        }

        let task_factory: TaskFactory<_> =
            Box::new(|chunk_id: ChunkId, emu_trace: Arc<EmuTrace>| {
                let data_bus = self.sm_bundle.build_data_bus_counters();
                CounterTask {
                    chunk_id,
                    emu_trace,
                    chunk_size: self.chunk_size,
                    data_bus,
                    zisk_rom: self.zisk_rom.clone(),
                    _phantom: std::marker::PhantomData::<F>,
                    _stats: self.stats.clone(),
                    #[cfg(feature = "stats")]
                    _parent_stats_id: parent_stats_id,
                    #[cfg(not(feature = "stats"))]
                    _parent_stats_id: 0,
                }
            });

        let (asm_runner_mt, mut data_buses) = AsmRunnerMT::run_and_count(
            self.asm_shmem_mt.lock().unwrap().as_mut().unwrap(),
            Self::MAX_NUM_STEPS,
            self.chunk_size,
            task_factory,
            self.world_rank,
            self.local_rank,
            self.base_port,
            self.stats.clone(),
        )
        .expect("Error during ASM execution");

        data_buses.sort_by_key(|(chunk_id, _)| chunk_id.0);

        let mut main_count = Vec::with_capacity(data_buses.len());
        let mut secn_count = Vec::with_capacity(data_buses.len());

        let main_idx = self.sm_bundle.main_counter_idx();
        for (chunk_id, data_bus) in data_buses {
            let databus_counters = data_bus.into_devices(false);

            let mut secondary = Vec::new();

            for (idx, (_, counter)) in databus_counters.into_iter().enumerate() {
                match main_idx {
                    None => secondary.push((chunk_id, counter)),
                    Some(i) if idx == i => {
                        main_count.push((chunk_id, counter.unwrap_or(Box::new(DummyCounter {}))))
                    }
                    Some(_) => secondary.push((chunk_id, counter)),
                }
            }

            secn_count.push(secondary);
        }

        // Group counters by chunk_id and counter type
        let mut secn_vec_counters =
            (0..secn_count[0].len()).map(|_| Vec::new()).collect::<Vec<_>>();

        secn_count.into_iter().for_each(|counter_slice| {
            counter_slice.into_iter().enumerate().for_each(|(i, (chunk_id, counter))| {
                secn_vec_counters[i].push((chunk_id, counter.unwrap_or(Box::new(DummyCounter {}))));
            });
        });

        (MinimalTraces::AsmEmuTrace(asm_runner_mt), main_count, secn_vec_counters)
    }

    fn run_emulator(&self, num_threads: usize, input_data_path: Option<PathBuf>) -> MinimalTraces {
        // Call emulate with these options
        let input_data = if let Some(path) = &input_data_path {
            // Read inputs data from the provided inputs path
            let path = PathBuf::from(path.display().to_string());
            fs::read(path).expect("Could not read inputs file")
        } else {
            Vec::new()
        };

        // Settings for the emulator
        let emu_options = EmuOptions {
            chunk_size: Some(self.chunk_size),
            max_steps: Self::MAX_NUM_STEPS,
            ..EmuOptions::default()
        };

        let min_traces = ZiskEmulator::compute_minimal_traces(
            &self.zisk_rom,
            &input_data,
            &emu_options,
            num_threads,
        )
        .expect("Error during emulator execution");

        MinimalTraces::EmuTrace(min_traces)
    }

    /// Adds main state machine instances to the proof context and assigns global IDs.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `main_planning` - Planning information for main state machines.
    fn assign_main_instances(&self, pctx: &ProofCtx<F>, main_planning: &mut [Plan]) {
        for plan in main_planning.iter_mut() {
            plan.set_global_id(pctx.add_instance_assign(
                plan.airgroup_id,
                plan.air_id,
                plan.n_threads_witness,
            ));
        }
    }

    /// Creates main state machine instance based on a main planning.
    ///
    /// # Arguments
    /// * `global_id` - Global ID of the main instance to be created.
    ///
    /// # Returns
    /// A main instance for the provided global ID.
    fn create_main_instance(&self, global_id: usize) -> MainInstance<F> {
        let mut main_planning_guard = self.main_planning.write().unwrap();

        let plan_idx = main_planning_guard
            .iter()
            .position(|x| x.global_id.unwrap() == global_id)
            .expect("Main instance not found");

        let plan = main_planning_guard.remove(plan_idx);

        let global_id = plan.global_id.unwrap();
        let is_last_segment = *plan
            .meta
            .as_ref()
            .and_then(|m| m.downcast_ref::<bool>())
            .unwrap_or_else(|| panic!("create_main_instance: Invalid metadata format"));

        MainInstance::new(InstanceCtx::new(global_id, plan), is_last_segment, self.std.clone())
    }

    /// Counts metrics for secondary state machines based on minimal traces.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces obtained from the ROM execution.
    ///
    /// # Returns
    /// A tuple containing two vectors:
    /// * A vector of main state machine metrics grouped by chunk ID.
    /// * A vector of secondary state machine metrics grouped by chunk ID. The vector is nested,
    ///   with the outer vector representing the secondary state machines and the inner vector
    ///   containing the metrics for each chunk.
    fn count(&self, min_traces: &MinimalTraces) -> (DeviceMetricsList, NestedDeviceMetricsList) {
        let min_traces = match min_traces {
            MinimalTraces::EmuTrace(min_traces) => min_traces,
            MinimalTraces::AsmEmuTrace(asm_min_traces) => &asm_min_traces.vec_chunks,
            _ => unreachable!(),
        };

        let (main_metrics_slices, secn_metrics_slices): (Vec<_>, Vec<_>) = min_traces
            .par_iter()
            .map(|minimal_trace| {
                let mut data_bus = self.sm_bundle.build_data_bus_counters();

                ZiskEmulator::process_emu_trace::<F, _, _>(
                    &self.zisk_rom,
                    minimal_trace,
                    &mut data_bus,
                    self.chunk_size,
                    true,
                );

                let (mut main_count, mut secn_count) = (Vec::new(), Vec::new());

                let databus_counters = data_bus.into_devices(true);
                let main_idx = self.sm_bundle.main_counter_idx();
                for (idx, counter) in databus_counters.into_iter().enumerate() {
                    match main_idx {
                        None => secn_count.push(counter),
                        Some(i) if idx == i => main_count.push(counter),
                        Some(_) => secn_count.push(counter),
                    }
                }
                (main_count, secn_count)
            })
            .unzip();

        // Group counters by chunk_id and counter type
        let mut secn_vec_counters =
            (0..secn_metrics_slices[0].len()).map(|_| Vec::new()).collect::<Vec<_>>();

        secn_metrics_slices.into_iter().enumerate().for_each(|(chunk_id, counter_slice)| {
            counter_slice.into_iter().enumerate().for_each(|(i, (_, counter))| {
                secn_vec_counters[i]
                    .push((ChunkId(chunk_id), counter.unwrap_or(Box::new(DummyCounter {}))));
            });
        });

        let main_vec_counters: Vec<_> = main_metrics_slices
            .into_iter()
            .enumerate()
            .flat_map(|(chunk_id, counters)| {
                counters.into_iter().map(move |(_, counter)| {
                    (ChunkId(chunk_id), counter.unwrap_or(Box::new(DummyCounter {})))
                })
            })
            .collect();

        (main_vec_counters, secn_vec_counters)
    }

    /// Adds secondary state machine instances to the proof context and assigns global IDs.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `secn_planning` - Planning information for secondary state machines.
    fn assign_secn_instances(&self, pctx: &ProofCtx<F>, secn_planning: &mut [Vec<Plan>]) {
        for plans_by_sm in secn_planning.iter_mut() {
            for plan in plans_by_sm.iter_mut() {
                // If the node has rank 0 and the plan targets the ROM instance,
                // we need to add it to the proof context using a special method.
                // This method allows us to mark it as an instance to be computed by node 0.
                let global_id = if plan.airgroup_id == ZISK_AIRGROUP_ID
                    && plan.air_id == ROM_AIR_IDS[0]
                {
                    // If this is the ROM instance, we need to add it to the proof context
                    // with the rank 0.
                    pctx.add_instance_assign_partition(
                        plan.airgroup_id,
                        plan.air_id,
                        0,
                        plan.n_threads_witness,
                    )
                } else {
                    match plan.instance_type {
                        InstanceType::Instance => {
                            pctx.add_instance(plan.airgroup_id, plan.air_id, plan.n_threads_witness)
                        }
                        InstanceType::Table => pctx.add_table(plan.airgroup_id, plan.air_id),
                    }
                };

                plan.set_global_id(global_id);
            }
        }
    }

    /// Creates a secondary state machine instance based on the provided global ID.
    ///
    /// # Arguments
    /// * `global_id` - Global ID of the secondary state machine instance.
    ///
    /// # Returns
    /// A secondary state machine instance for the provided global ID.
    fn create_secn_instance(&self, global_id: usize) -> Box<dyn Instance<F>> {
        let mut secn_planning_guard = self.secn_planning.write().unwrap();

        let plan_idx = secn_planning_guard.iter().enumerate().find_map(|(outer_idx, plans)| {
            plans
                .iter()
                .position(|plan| plan.global_id.unwrap() == global_id)
                .map(|inner_idx| (outer_idx, inner_idx))
        });
        if plan_idx.is_none() {
            panic!("Secondary instance not found");
        }

        let plan_idx = plan_idx.unwrap();
        let plan = secn_planning_guard[plan_idx.0].remove(plan_idx.1);

        let global_id = plan.global_id.unwrap();

        let ictx = InstanceCtx::new(global_id, plan);
        self.sm_bundle.build_instance(plan_idx.0, ictx)
    }

    /// Expands and computes witnesses for a main instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `main_instance` - Main instance to compute witness for
    fn witness_main_instance(
        &self,
        pctx: &ProofCtx<F>,
        main_instance: &MainInstance<F>,
        trace_buffer: Vec<F>,
        _caller_stats_id: u64,
    ) {
        let (airgroup_id, air_id) = pctx.dctx_get_instance_info(main_instance.ictx.global_id);
        let witness_start_time = Instant::now();

        #[cfg(feature = "stats")]
        let stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            _caller_stats_id,
            stats_id,
            "AIR_MAIN_WITNESS",
            air_id,
            ExecutorStatsEvent::Begin,
        );

        let min_traces_guard = self.min_traces.read().unwrap();
        let min_traces = &*min_traces_guard;

        let min_traces = match min_traces {
            MinimalTraces::EmuTrace(min_traces) => min_traces,
            MinimalTraces::AsmEmuTrace(asm_min_traces) => &asm_min_traces.vec_chunks,
            _ => unreachable!(),
        };

        let air_instance = main_instance.compute_witness(
            &self.zisk_rom,
            min_traces,
            self.chunk_size,
            main_instance,
            trace_buffer,
        );

        pctx.add_air_instance(air_instance, main_instance.ictx.global_id);

        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            _caller_stats_id,
            stats_id,
            "AIR_MAIN_WITNESS",
            air_id,
            ExecutorStatsEvent::End,
        );

        let stats = Stats {
            airgroup_id,
            air_id,
            collect_start_time: Instant::now(),
            collect_duration: 0,
            witness_start_time: Instant::now(),
            witness_duration: witness_start_time.elapsed().as_millis() as u64,
            num_chunks: 0,
        };

        self.witness_stats.lock().unwrap().insert(main_instance.ictx.global_id, stats);
    }

    /// computes witness for a secondary state machines instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_id` - Global ID of the secondary state machine instance.
    /// * `secn_instance` - Secondary state machine instance to compute witness for
    fn witness_secn_instance(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        secn_instance: &dyn Instance<F>,
        trace_buffer: Vec<F>,
        _caller_stats_id: u64,
    ) {
        let witness_start_time = Instant::now();

        #[cfg(feature = "stats")]
        let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id);
        #[cfg(feature = "stats")]
        let stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            _caller_stats_id,
            stats_id,
            "AIR_SECN_WITNESS",
            air_id,
            ExecutorStatsEvent::Begin,
        );

        let collectors_by_instance = {
            let mut guard = self.collectors_by_instance.write().unwrap();

            guard.remove(&global_id).expect("Missing collectors for given global_id")
        };

        if let Some(air_instance) =
            secn_instance.compute_witness(pctx, sctx, collectors_by_instance, trace_buffer)
        {
            pctx.add_air_instance(air_instance, global_id);
        }
        #[cfg(feature = "stats")]
        {
            self.stats.lock().unwrap().add_stat(
                _caller_stats_id,
                stats_id,
                "AIR_SECN_WITNESS",
                air_id,
                ExecutorStatsEvent::End,
            );
        }
        self.witness_stats.lock().unwrap().get_mut(&global_id).unwrap().witness_duration =
            witness_start_time.elapsed().as_millis() as u64;
    }

    fn order_chunks(
        &self,
        chunks_to_execute: &[Vec<usize>],
        global_id_chunks: &HashMap<usize, Vec<usize>>,
    ) -> Vec<usize> {
        let mut ordered_chunks = Vec::new();
        let mut already_selected_chunks = vec![false; chunks_to_execute.len()];

        let mut n_global_ids_incompleted = global_id_chunks.len();
        let mut n_chunks_by_global_id: HashMap<usize, usize> =
            global_id_chunks.iter().map(|(global_id, chunks)| (*global_id, chunks.len())).collect();

        while n_global_ids_incompleted > 0 {
            let selected_global_id = n_chunks_by_global_id
                .iter()
                .filter(|(_, &count)| count > 0)
                .min_by_key(|(_, &count)| count)
                .map(|(&global_id, _)| global_id);

            if let Some(global_id) = selected_global_id {
                for chunk_id in global_id_chunks[&global_id].iter() {
                    if already_selected_chunks[*chunk_id] {
                        continue;
                    }
                    ordered_chunks.push(*chunk_id);
                    already_selected_chunks[*chunk_id] = true;
                    for global_idx in chunks_to_execute[*chunk_id].iter() {
                        if let Some(count) = n_chunks_by_global_id.get_mut(global_idx) {
                            *count -= 1;
                            if *count == 0 {
                                n_chunks_by_global_id.remove(global_idx);
                                n_global_ids_incompleted -= 1;
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        ordered_chunks
    }

    /// Expands for a secondary state machines instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_id` - Global ID of the secondary state machine instance.
    /// * `secn_instance` - Secondary state machine instance to compute witness for
    #[allow(clippy::borrowed_box)]
    fn witness_collect_instances(
        &self,
        pctx: Arc<ProofCtx<F>>,
        secn_instances: HashMap<usize, &Box<dyn Instance<F>>>,
    ) {
        let min_traces = self.min_traces.read().unwrap();

        let min_traces = match &*min_traces {
            MinimalTraces::EmuTrace(min_traces) => min_traces,
            MinimalTraces::AsmEmuTrace(asm_min_traces) => &asm_min_traces.vec_chunks,
            _ => unreachable!(),
        };

        // Group the instances by the chunk they need to process
        let (chunks_to_execute, global_id_chunks) =
            self.chunks_to_execute(min_traces, &secn_instances);

        let ordered_chunks = self.order_chunks(&chunks_to_execute, &global_id_chunks);
        let global_ids: Vec<usize> = secn_instances.keys().copied().collect();

        let collect_start_times: Vec<Arc<AtomicCell<Option<Instant>>>> =
            global_ids.iter().map(|_| Arc::new(AtomicCell::new(None))).collect();

        let chunks_to_execute_clone = chunks_to_execute.clone();

        // Pre-allocate lock-free queues for each global_id
        #[allow(clippy::type_complexity)]
        let collectors_queues: Vec<Arc<SegQueue<(usize, Box<dyn BusDevice<u64>>)>>> =
            global_ids.iter().map(|_| Arc::new(SegQueue::new())).collect();

        let global_ids_map: HashMap<usize, usize> =
            global_ids.iter().enumerate().map(|(idx, &id)| (id, idx)).collect();

        // Create data buses for each chunk
        let data_buses = self
            .sm_bundle
            .build_data_bus_collectors(&secn_instances, chunks_to_execute)
            .into_iter()
            .map(|db| Arc::new(Mutex::new(db)))
            .collect::<Vec<_>>();

        let n_chunks_left: Vec<Arc<AtomicUsize>> = global_ids
            .iter()
            .map(|global_id| Arc::new(AtomicUsize::new(global_id_chunks[global_id].len())))
            .collect();

        for global_id in global_ids.iter() {
            let (airgroup_id, air_id) = pctx.dctx_get_instance_info(*global_id);
            let stats = Stats {
                airgroup_id,
                air_id,
                collect_start_time: Instant::now(),
                collect_duration: 0,
                witness_start_time: Instant::now(),
                witness_duration: 0,
                num_chunks: global_id_chunks[global_id].len(),
            };

            self.collectors_by_instance.write().unwrap().insert(*global_id, Vec::new());
            self.witness_stats.lock().unwrap().insert(*global_id, stats);
        }

        let next_chunk = Arc::new(AtomicUsize::new(0));
        let n_threads = rayon::current_num_threads();

        let mut handles = Vec::with_capacity(n_threads);
        for _ in 0..n_threads {
            let next_chunk = Arc::clone(&next_chunk);
            let min_traces_lock = Arc::clone(&self.min_traces);
            let data_buses = data_buses.clone();
            let zisk_rom = self.zisk_rom.clone();
            let collectors_queues = collectors_queues.clone();
            let n_chunks_left = n_chunks_left.clone();
            let global_ids_map = global_ids_map.clone();
            let global_id_chunks = global_id_chunks.clone();
            let collectors_by_instance = self.collectors_by_instance.clone();
            let witness_stats = self.witness_stats.clone();
            let ordered_chunks_clone = ordered_chunks.clone();
            let chunk_size = self.chunk_size;

            let pctx_clone = pctx.clone();

            let chunks_to_execute = chunks_to_execute_clone.clone();

            let collect_start_times = collect_start_times.clone();

            handles.push(std::thread::spawn(move || {
                let guard = min_traces_lock.read().unwrap();
                let min_traces = match &*guard {
                    MinimalTraces::EmuTrace(v) => v,
                    MinimalTraces::AsmEmuTrace(a) => &a.vec_chunks,
                    _ => unreachable!(),
                };
                loop {
                    let next_chunk_id = next_chunk.fetch_add(1, Ordering::SeqCst);
                    if next_chunk_id >= ordered_chunks_clone.len() {
                        break;
                    }
                    let chunk_id = ordered_chunks_clone[next_chunk_id];

                    if let Some(mut data_bus) = data_buses[chunk_id].lock().unwrap().take() {
                        for global_id in chunks_to_execute[chunk_id].iter() {
                            let start_time_cell = &collect_start_times[global_ids_map[global_id]];
                            if start_time_cell.load().is_none() {
                                start_time_cell.store(Some(Instant::now()));
                            }
                        }

                        ZiskEmulator::process_emu_traces::<F, _, _>(
                            &zisk_rom,
                            min_traces,
                            chunk_id,
                            &mut data_bus,
                            chunk_size,
                        );

                        for (global_id, collector) in data_bus.into_devices(false) {
                            if let Some(global_id) = global_id {
                                let global_id_idx = global_ids_map
                                    .get(&global_id)
                                    .expect("Global ID not found in map");

                                collectors_queues[*global_id_idx]
                                    .push((chunk_id, collector.unwrap()));

                                if n_chunks_left[*global_id_idx].fetch_sub(1, Ordering::SeqCst) == 1
                                {
                                    let mut collectors = Vec::new();
                                    while let Some((chunk_id, collector)) =
                                        collectors_queues[*global_id_idx].pop()
                                    {
                                        collectors.push((chunk_id, collector));
                                    }

                                    // Sort collectors by chunk_id based on the order in global_id_chunks
                                    let chunk_order = &global_id_chunks[&global_id];
                                    collectors.sort_by_key(|(chunk_id, _)| {
                                        chunk_order.iter().position(|&id| id == *chunk_id).unwrap()
                                    });

                                    let collect_start_time = collect_start_times[*global_id_idx]
                                        .load()
                                        .expect("Collect start time was not set");
                                    let collect_duration =
                                        collect_start_time.elapsed().as_millis() as u64;

                                    let (airgroup_id, air_id) =
                                        pctx_clone.dctx_get_instance_info(global_id);
                                    let stats = Stats {
                                        airgroup_id,
                                        air_id,
                                        collect_start_time,
                                        collect_duration,
                                        witness_start_time: Instant::now(),
                                        witness_duration: 0,
                                        num_chunks: global_id_chunks[&global_id].len(),
                                    };

                                    witness_stats.lock().unwrap().insert(global_id, stats);

                                    collectors_by_instance
                                        .write()
                                        .unwrap()
                                        .insert(global_id, collectors);

                                    pctx_clone.set_witness_ready(global_id, true);
                                }
                            }
                        }
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    /// Computes and generates witness for secondary state machine instance of type `Table`.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_id` - Global ID of the secondary state machine instance.
    /// * `table_instance` - Secondary state machine table instance to compute witness for
    fn witness_table(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        table_instance: &dyn Instance<F>,
        trace_buffer: Vec<F>,
        _caller_stats_id: u64,
    ) {
        #[cfg(feature = "stats")]
        let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id);
        #[cfg(feature = "stats")]
        let stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            _caller_stats_id,
            stats_id,
            "AIR_WITNESS_TABLE",
            air_id,
            ExecutorStatsEvent::Begin,
        );
        assert_eq!(table_instance.instance_type(), InstanceType::Table, "Instance is not a table");

        if let Some(air_instance) = table_instance.compute_witness(pctx, sctx, vec![], trace_buffer)
        {
            if pctx.dctx_is_my_process_instance(global_id) {
                pctx.add_air_instance(air_instance, global_id);
            }
        }

        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            _caller_stats_id,
            stats_id,
            "AIR_WITNESS_TABLE",
            air_id,
            ExecutorStatsEvent::Begin,
        );
    }

    /// Computes all the chunks to be executed to generate the witness given an instance.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces
    /// * `secn_instance` - Secondary state machine instance to group.
    ///
    /// # Returns
    /// A vector of booleans indicating which chunks to execute.
    #[allow(clippy::borrowed_box)]
    fn chunks_to_execute(
        &self,
        min_traces: &[EmuTrace],
        secn_instances: &HashMap<usize, &Box<dyn Instance<F>>>,
    ) -> (Vec<Vec<usize>>, HashMap<usize, Vec<usize>>) {
        let mut chunks_to_execute = vec![Vec::new(); min_traces.len()];
        let mut global_id_chunks: HashMap<usize, Vec<usize>> = HashMap::new();
        secn_instances.iter().for_each(|(global_idx, secn_instance)| {
            match secn_instance.check_point() {
                CheckPoint::None => {}
                CheckPoint::Single(chunk_id) => {
                    chunks_to_execute[chunk_id.as_usize()].push(*global_idx);
                    global_id_chunks.entry(*global_idx).or_default().push(chunk_id.as_usize());
                }
                CheckPoint::Multiple(chunk_ids) => {
                    chunk_ids.iter().for_each(|&chunk_id| {
                        chunks_to_execute[chunk_id.as_usize()].push(*global_idx);
                        global_id_chunks.entry(*global_idx).or_default().push(chunk_id.as_usize());
                    });
                }
            }
        });

        for chunk_ids in global_id_chunks.values_mut() {
            chunk_ids.sort();
        }

        (chunks_to_execute, global_id_chunks)
    }

    fn reset(&self) {
        // Reset the internal state of the executor
        *self.execution_result.lock().unwrap() = ZiskExecutionResult::default();
        *self.min_traces.write().unwrap() = MinimalTraces::None;
        *self.main_planning.write().unwrap() = Vec::new();
        *self.secn_planning.write().unwrap() = Vec::new();
        self.main_instances.write().unwrap().clear();
        self.secn_instances.write().unwrap().clear();
        self.collectors_by_instance.write().unwrap().clear();
        self.stats.lock().unwrap().reset();
    }
}

impl<F: PrimeField64, BD: SMBundle<F>> WitnessComponent<F> for ZiskExecutor<F, BD> {
    /// Executes the ZisK ROM program and calculate the plans for main and secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    ///
    /// # Returns
    /// A vector of global IDs for the instances to compute witness for.
    fn execute(&self, pctx: Arc<ProofCtx<F>>, input_data_path: Option<PathBuf>) -> Vec<usize> {
        #[cfg(feature = "stats")]
        let parent_stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "EXECUTE",
            0,
            ExecutorStatsEvent::Begin,
        );

        self.reset();

        // Set the start time of the current execution
        self.stats.lock().unwrap().set_start_time(Instant::now());

        // Process the ROM to collect the Minimal Traces
        timer_start_info!(COMPUTE_MINIMAL_TRACE);

        assert_eq!(self.asm_runner_path.is_some(), self.asm_rom_path.is_some());

        let (min_traces, main_count, secn_count, handle_mo) = if self.asm_runner_path.is_some() {
            // If we are executing in assembly mode
            self.execute_with_assembly(
                input_data_path,
                #[cfg(feature = "stats")]
                parent_stats_id,
                #[cfg(not(feature = "stats"))]
                0,
            )
        } else {
            // Otherwise, use the emulator
            let min_traces = self.execute_with_emulator(input_data_path);

            timer_start_info!(COUNT);
            let (main_count, secn_count) = self.count(&min_traces);
            timer_stop_and_log_info!(COUNT);

            (min_traces, main_count, secn_count, None)
        };
        timer_stop_and_log_info!(COMPUTE_MINIMAL_TRACE);

        // Plan the main and secondary instances using the counted metrics
        #[cfg(feature = "stats")]
        let stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            parent_stats_id,
            stats_id,
            "MAIN_PLAN",
            0,
            ExecutorStatsEvent::Begin,
        );

        timer_start_info!(PLAN);
        let (mut main_planning, public_values) =
            MainPlanner::plan::<F>(&min_traces, main_count, self.chunk_size);

        // Add to executor stats
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            parent_stats_id,
            stats_id,
            "MAIN_PLAN",
            0,
            ExecutorStatsEvent::End,
        );
        #[cfg(feature = "stats")]
        let stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            parent_stats_id,
            stats_id,
            "SECN_PLAN",
            0,
            ExecutorStatsEvent::Begin,
        );

        let mut secn_planning = self.sm_bundle.plan_sec(secn_count);

        // Add to executor stats
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            parent_stats_id,
            stats_id,
            "SECN_PLAN",
            0,
            ExecutorStatsEvent::End,
        );

        if let Some(handle_mo) = handle_mo {
            #[cfg(feature = "stats")]
            let stats_id = self.stats.lock().unwrap().get_id();
            #[cfg(feature = "stats")]
            self.stats.lock().unwrap().add_stat(
                parent_stats_id,
                stats_id,
                "MO_PLAN_WAIT",
                0,
                ExecutorStatsEvent::Begin,
            );

            // Wait for the memory operations thread to finish
            let asm_runner_mo =
                handle_mo.join().expect("Error during Assembly Memory Operations thread execution");

            // Add to executor stats
            #[cfg(feature = "stats")]
            self.stats.lock().unwrap().add_stat(
                parent_stats_id,
                stats_id,
                "MO_PLAN_WAIT",
                0,
                ExecutorStatsEvent::End,
            );
            #[cfg(feature = "stats")]
            let stats_id = self.stats.lock().unwrap().get_id();
            #[cfg(feature = "stats")]
            self.stats.lock().unwrap().add_stat(
                parent_stats_id,
                stats_id,
                "MO_PLAN_ADD",
                0,
                ExecutorStatsEvent::Begin,
            );

            secn_planning[0].extend(asm_runner_mo.plans);

            // Add to executor stats
            #[cfg(feature = "stats")]
            self.stats.lock().unwrap().add_stat(
                parent_stats_id,
                stats_id,
                "MO_PLAN_ADD",
                0,
                ExecutorStatsEvent::End,
            );
        }

        timer_stop_and_log_info!(PLAN);

        #[cfg(feature = "stats")]
        let stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            parent_stats_id,
            stats_id,
            "CONFIGURE_INSTANCES",
            0,
            ExecutorStatsEvent::Begin,
        );

        // Configure the instances
        self.sm_bundle.configure_instances(&pctx, &secn_planning);

        // Assign the instances
        self.assign_main_instances(&pctx, &mut main_planning);
        self.assign_secn_instances(&pctx, &mut secn_planning);

        // Get the global IDs of the instances to compute witness for
        let main_global_ids =
            main_planning.iter().map(|plan| plan.global_id.unwrap()).collect::<Vec<_>>();
        let secn_global_ids = secn_planning
            .iter()
            .map(|plans| plans.iter().map(|plan| plan.global_id.unwrap()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let secn_global_ids_vec = secn_global_ids.iter().flatten().copied().collect::<Vec<_>>();

        // Add public values to the proof context
        let mut publics = ZiskPublicValues::from_vec_guard(pctx.get_publics());
        for (index, value) in public_values.iter() {
            publics.inputs[*index as usize] = F::from_u32(*value);
        }
        drop(publics);

        // Update internal state with the computed minimal traces and planning.
        *self.min_traces.write().unwrap() = min_traces;
        *self.main_planning.write().unwrap() = main_planning;
        *self.secn_planning.write().unwrap() = secn_planning;

        let mut main_instances = self.main_instances.write().unwrap();

        for global_id in &main_global_ids {
            main_instances
                .entry(*global_id)
                .or_insert_with(|| self.create_main_instance(*global_id));
        }

        let mut secn_instances = self.secn_instances.write().unwrap();
        for global_id in &secn_global_ids_vec {
            secn_instances
                .entry(*global_id)
                .or_insert_with(|| self.create_secn_instance(*global_id));
            secn_instances[global_id].reset();
        }

        // Add to executor stats
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            parent_stats_id,
            stats_id,
            "CONFIGURE_INSTANCES",
            0,
            ExecutorStatsEvent::End,
        );

        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "EXECUTE",
            0,
            ExecutorStatsEvent::End,
        );

        // #[cfg(feature = "stats")]
        // self.stats.lock().unwrap().store_stats();

        [main_global_ids, secn_global_ids_vec].concat()
    }

    /// Computes the witness for the main and secondary state machines.
    ///
    /// # Arguments
    /// * `stage` - The current stage id
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_ids` - Global IDs of the instances to compute witness for.
    fn calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        buffer_pool: &dyn BufferPool<F>,
    ) {
        #[cfg(feature = "stats")]
        let parent_stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "CALCULATE_WITNESS",
            0,
            ExecutorStatsEvent::Begin,
        );

        if stage != 1 {
            return;
        }

        let pool = create_pool(n_cores);
        pool.install(|| {
            for &global_id in global_ids {
                let (airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id);

                if MAIN_AIR_IDS.contains(&air_id) {
                    let main_instance = &self.main_instances.read().unwrap()[&global_id];

                    self.witness_main_instance(
                        &pctx,
                        main_instance,
                        buffer_pool.take_buffer(),
                        #[cfg(feature = "stats")]
                        parent_stats_id,
                        #[cfg(not(feature = "stats"))]
                        0,
                    );
                } else {
                    let secn_instance = &self.secn_instances.read().unwrap()[&global_id];

                    match secn_instance.instance_type() {
                        InstanceType::Instance => {
                            if air_id == ROM_AIR_IDS[0] && self.asm_runner_path.is_some() {
                                let stats = Stats {
                                    airgroup_id,
                                    air_id,
                                    collect_start_time: Instant::now(),
                                    collect_duration: 0,
                                    witness_start_time: Instant::now(),
                                    witness_duration: 0,
                                    num_chunks: 0,
                                };

                                self.collectors_by_instance
                                    .write()
                                    .unwrap()
                                    .insert(global_id, Vec::new());
                                self.witness_stats.lock().unwrap().insert(global_id, stats);
                            } else if !self
                                .collectors_by_instance
                                .read()
                                .unwrap()
                                .contains_key(&global_id)
                            {
                                let mut secn_instances = HashMap::new();
                                secn_instances.insert(global_id, secn_instance);
                                self.witness_collect_instances(pctx.clone(), secn_instances);
                            }
                            self.witness_secn_instance(
                                &pctx,
                                &sctx,
                                global_id,
                                &**secn_instance,
                                buffer_pool.take_buffer(),
                                #[cfg(feature = "stats")]
                                parent_stats_id,
                                #[cfg(not(feature = "stats"))]
                                0,
                            );
                        }
                        InstanceType::Table => self.witness_table(
                            &pctx,
                            &sctx,
                            global_id,
                            &**secn_instance,
                            Vec::new(),
                            #[cfg(feature = "stats")]
                            parent_stats_id,
                            #[cfg(not(feature = "stats"))]
                            0,
                        ),
                    }
                }
            }
        });

        // Add to executor stats
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "CALCULATE_WITNESS",
            0,
            ExecutorStatsEvent::End,
        );
    }

    fn pre_calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        _sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        _buffer_pool: &dyn BufferPool<F>,
    ) {
        #[cfg(feature = "stats")]
        let parent_stats_id = self.stats.lock().unwrap().get_id();
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "PRE_CALCULATE_WITNESS",
            0,
            ExecutorStatsEvent::Begin,
        );

        if stage != 1 {
            return;
        }
        let secn_instances_guard = self.secn_instances.read().unwrap();

        let mut secn_instances = HashMap::new();
        let mut secn_instances_memory = HashMap::new();
        for &global_id in global_ids {
            let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id);
            if MAIN_AIR_IDS.contains(&air_id)
                || (air_id == ROM_AIR_IDS[0] && self.asm_runner_path.is_some())
            {
                pctx.set_witness_ready(global_id, false);
            } else {
                let secn_instance = &secn_instances_guard[&global_id];

                if secn_instance.instance_type() == InstanceType::Instance
                    && !self.collectors_by_instance.read().unwrap().contains_key(&global_id)
                {
                    if air_id == MEM_AIR_IDS[0] || air_id == ROM_DATA_AIR_IDS[0] || air_id == INPUT_DATA_AIR_IDS[0] {
                        secn_instances_memory.insert(global_id, secn_instance);
                    } else {
                        secn_instances.insert(global_id, secn_instance);
                    }
                } else {
                    pctx.set_witness_ready(global_id, true);
                }
            }
        }

        let pool = create_pool(n_cores);
        pool.install(|| {
            if !secn_instances.is_empty() {
                timer_start_info!(PRE_CALCULATE_WITNESS_SEC);
                self.witness_collect_instances(pctx.clone(), secn_instances);
                timer_stop_and_log_info!(PRE_CALCULATE_WITNESS_SEC);
            }

            if !secn_instances_memory.is_empty() {
                timer_start_info!(PRE_CALCULATE_WITNESS_MEMORY);
                self.witness_collect_instances(pctx.clone(), secn_instances_memory);
                timer_stop_and_log_info!(PRE_CALCULATE_WITNESS_MEMORY);
            }
        });

        // Add to executor stats
        #[cfg(feature = "stats")]
        self.stats.lock().unwrap().add_stat(
            0,
            parent_stats_id,
            "PRE_CALCULATE_WITNESS",
            0,
            ExecutorStatsEvent::End,
        );
    }

    /// Debugs the main and secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_ids` - Global IDs of the instances to debug.
    fn debug(&self, pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx<F>>, global_ids: &[usize]) {
        for &global_id in global_ids {
            let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id);

            if MAIN_AIR_IDS.contains(&air_id) {
                MainSM::debug(&pctx, &sctx);
            } else {
                let secn_instances = self.secn_instances.read().unwrap();
                let secn_instance = secn_instances.get(&global_id).expect("Instance not found");

                secn_instance.debug(&pctx, &sctx);
            }
        }
    }

    fn gen_custom_commits_fixed(
        &self,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        check: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_name = pctx.get_custom_commits_fixed_buffer("rom", false)?;

        let setup = sctx.get_setup(RomRomTrace::<usize>::AIRGROUP_ID, RomRomTrace::<usize>::AIR_ID);
        let blowup_factor =
            1 << (setup.stark_info.stark_struct.n_bits_ext - setup.stark_info.stark_struct.n_bits);

        gen_elf_hash(&self.rom_path, file_name.as_path(), blowup_factor, check)?;
        Ok(())
    }
}
