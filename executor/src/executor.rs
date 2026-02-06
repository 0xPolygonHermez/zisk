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

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{create_pool, BufferPool, ProofCtx, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use sm_rom::RomInstance;
use std::sync::atomic::{AtomicUsize, Ordering};
use witness::WitnessComponent;
use zisk_common::io::{StreamSource, ZiskStdin, ZiskStream};

use data_bus::DataBusTrait;
use sm_main::{MainInstance, MainPlanner, MainSM};
use zisk_common::{
    stats_begin, stats_end, BusDevice, BusDeviceMetrics, CheckPoint, ChunkId, EmuTrace,
    ExecutorStatsHandle, Instance, InstanceCtx, InstanceType, Plan, Stats, ZiskExecutionResult,
};
use zisk_pil::{
    ZiskPublicValues, INPUT_DATA_AIR_IDS, MAIN_AIR_IDS, MEM_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

use std::time::Instant;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use crossbeam::atomic::AtomicCell;

use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use crate::{Emulator, EmulatorKind, StaticSMBundle};

use anyhow::Result;

pub type DeviceMetricsByChunk = (ChunkId, Box<dyn BusDeviceMetrics>); // (chunk_id, metrics)
type ChunkCollector = (usize, Box<dyn BusDevice<u64>>);

#[allow(dead_code)]
enum MinimalTraceExecutionMode {
    Emulator,
    AsmWithCounter,
}

/// The maximum number of steps to execute in the emulator or assembly runner.
pub const MAX_NUM_STEPS: u64 = 1 << 36;

/// The `ZiskExecutor` struct orchestrates the execution of the ZisK ROM program, managing state
/// machines, planning, and witness computation.
pub struct ZiskExecutor<F: PrimeField64> {
    /// Standard input for the ZisK program execution.
    stdin: Mutex<ZiskStdin>,

    /// The emulator backend used for execution.
    emulator: EmulatorKind,

    /// Chunk size for processing.
    chunk_size: u64,

    /// Pipeline for handling precompile hints.
    hints_stream: Mutex<Option<ZiskStream>>,

    /// ZisK ROM, a binary file containing the ZisK program to be executed.
    zisk_rom: Arc<ZiskRom>,

    /// Planning information for main state machines.
    min_traces: Arc<RwLock<Option<Vec<EmuTrace>>>>,

    /// Planning information for secondary state machines.
    secn_planning: RwLock<Vec<Plan>>,

    /// Main state machine instances, indexed by their global ID.
    main_instances: RwLock<HashMap<usize, MainInstance<F>>>,

    /// Secondary state machine instances, indexed by their global ID.
    secn_instances: RwLock<HashMap<usize, Box<dyn Instance<F>>>>,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,

    /// Execution result, including the number of executed steps.
    execution_result: Mutex<ZiskExecutionResult>,

    /// State machine bundle, containing the state machines and their configurations.
    sm_bundle: StaticSMBundle<F>,

    /// Collectors by instance, storing statistics and collectors for each instance.
    collectors_by_instance: Arc<RwLock<HashMap<usize, Vec<Option<ChunkCollector>>>>>,

    /// Statistics collected during the execution, including time taken for collection and witness computation.
    stats: ExecutorStatsHandle,
}

impl<F: PrimeField64> ZiskExecutor<F> {
    /// Creates a new instance of the `ZiskExecutor`.
    ///
    /// # Arguments
    /// * `zisk_rom` - An `Arc`-wrapped ZisK ROM instance.
    /// * `hints_stream` - Optional hints stream for processing precompile hints.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        zisk_rom: Arc<ZiskRom>,
        std: Arc<Std<F>>,
        sm_bundle: StaticSMBundle<F>,
        chunk_size: u64,
        emulator: EmulatorKind,
        hints_stream: Option<ZiskStream>,
    ) -> Self {
        Self {
            stdin: Mutex::new(ZiskStdin::null()),
            emulator,
            chunk_size,
            hints_stream: Mutex::new(hints_stream),
            zisk_rom,
            min_traces: Arc::new(RwLock::new(None)),
            secn_planning: RwLock::new(Vec::new()),
            main_instances: RwLock::new(HashMap::new()),
            secn_instances: RwLock::new(HashMap::new()),
            collectors_by_instance: Arc::new(RwLock::new(HashMap::new())),
            std,
            execution_result: Mutex::new(ZiskExecutionResult::default()),
            sm_bundle,
            stats: ExecutorStatsHandle::new(),
        }
    }

    pub fn set_stdin(&self, stdin: ZiskStdin) {
        let mut guard = self.stdin.lock().unwrap();
        *guard = stdin;
    }

    pub fn set_hints_stream_src(&self, stream: StreamSource) -> Result<()> {
        if let Some(hints_stream) = self.hints_stream.lock().unwrap().as_mut() {
            hints_stream.set_hints_stream_src(stream)
        } else {
            Err(anyhow::anyhow!("No hints stream configured"))
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn get_execution_result(&self) -> (ZiskExecutionResult, ExecutorStatsHandle) {
        (self.execution_result.lock().unwrap().clone(), self.stats.clone())
    }

    pub fn store_stats(&self) {
        self.stats.store_stats();
    }

    /// Adds main state machine instances to the proof context and assigns global IDs.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `main_planning` - Planning information for main state machines.
    fn assign_main_instances(
        &self,
        pctx: &ProofCtx<F>,
        global_ids: &RwLock<Vec<usize>>,
        main_planning: Vec<Plan>,
    ) {
        let mut main_instances = self.main_instances.write().unwrap();

        for mut plan in main_planning {
            let global_id = pctx
                .add_instance_assign(plan.airgroup_id, plan.air_id)
                .expect("Failed to add instance");
            plan.set_global_id(global_id);
            global_ids.write().unwrap().push(global_id);
            main_instances
                .entry(global_id)
                .or_insert_with(|| self.create_main_instance(plan, global_id));
        }
    }

    /// Creates main state machine instance based on a main planning.
    ///
    /// # Arguments
    /// * `global_id` - Global ID of the main instance to be created.
    ///
    /// # Returns
    /// A main instance for the provided global ID.
    fn create_main_instance(&self, plan: Plan, global_id: usize) -> MainInstance<F> {
        MainInstance::new(InstanceCtx::new(global_id, plan), self.std.clone())
    }

    /// Adds secondary state machine instances to the proof context and assigns global IDs.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `secn_planning` - Planning information for secondary state machines.
    fn assign_secn_instances(
        &self,
        pctx: &ProofCtx<F>,
        global_ids: &RwLock<Vec<usize>>,
        secn_planning: &mut [Plan],
    ) {
        for plan in secn_planning.iter_mut() {
            // If the node has rank 0 and the plan targets the ROM instance,
            // we need to add it to the proof context using a special method.
            // This method allows us to mark it as an instance to be computed by node 0.
            let global_id = if plan.airgroup_id == ZISK_AIRGROUP_ID && plan.air_id == ROM_AIR_IDS[0]
            {
                // If this is the ROM instance, we need to add it to the proof context
                // with the rank 0.
                pctx.add_instance_assign_first_partition(plan.airgroup_id, plan.air_id)
                    .expect("Failed to add ROM instance")
            } else {
                match plan.instance_type {
                    InstanceType::Instance => pctx
                        .add_instance(plan.airgroup_id, plan.air_id)
                        .expect("Failed to add instance"),
                    InstanceType::Table => {
                        pctx.add_table(plan.airgroup_id, plan.air_id).expect("Failed to add table")
                    }
                }
            };

            global_ids.write().unwrap().push(global_id);
            plan.set_global_id(global_id);
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

        let plan_idx =
            secn_planning_guard.iter().position(|plan| plan.global_id.unwrap() == global_id);
        if plan_idx.is_none() {
            panic!("Secondary instance not found");
        }

        let plan_idx = plan_idx.unwrap();
        let plan = secn_planning_guard.remove(plan_idx);

        let global_id = plan.global_id.unwrap();

        let ictx = InstanceCtx::new(global_id, plan);
        self.sm_bundle.build_instance(ictx)
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
    ) -> ProofmanResult<()> {
        let (airgroup_id, air_id) = pctx
            .dctx_get_instance_info(main_instance.ictx.global_id)
            .expect("Failed to get instance info");
        let witness_start_time = Instant::now();

        stats_begin!(self.stats, _caller_stats_id, _stats_scope, "AIR_MAIN_WITNESS", air_id);

        let min_traces_guard = self.min_traces.read().unwrap();
        let min_traces = min_traces_guard.as_ref().expect("min_traces should not be None");

        let air_instance = main_instance.compute_witness(
            &self.zisk_rom,
            min_traces,
            self.chunk_size,
            main_instance,
            trace_buffer,
        )?;

        pctx.add_air_instance(air_instance, main_instance.ictx.global_id);

        stats_end!(self.stats, &_stats_scope);

        let stats = Stats {
            airgroup_id,
            air_id,
            collect_start_time: Instant::now(),
            collect_duration: 0,
            witness_start_time: Instant::now(),
            witness_duration: witness_start_time.elapsed().as_millis(),
            num_chunks: 0,
        };

        self.stats.insert_witness_stats(main_instance.ictx.global_id, stats);

        Ok(())
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
    ) -> ProofmanResult<()> {
        let witness_start_time = Instant::now();

        #[cfg(feature = "stats")]
        let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id)?;
        stats_begin!(self.stats, _caller_stats_id, _stats_scope, "AIR_SECN_WITNESS", air_id);

        let collectors_by_instance = {
            let mut guard = self.collectors_by_instance.write().unwrap();

            guard
                .remove(&global_id)
                .expect("Missing collectors for given global_id")
                .into_iter()
                .enumerate()
                .map(|(idx, opt)| {
                    opt.unwrap_or_else(|| {
                        panic!("Collector at index {} for global_id {} is None", idx, global_id)
                    })
                })
                .collect()
        };

        if let Some(air_instance) =
            secn_instance.compute_witness(pctx, sctx, collectors_by_instance, trace_buffer)?
        {
            pctx.add_air_instance(air_instance, global_id);
        }

        stats_end!(self.stats, &_stats_scope);

        self.stats.set_witness_duration(global_id, witness_start_time.elapsed().as_millis());
        Ok(())
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
        let min_traces_guard = self.min_traces.read().unwrap();
        let min_traces = min_traces_guard.as_ref().expect("min_traces should not be None");

        // Group the instances by the chunk they need to process
        let (chunks_to_execute, global_id_chunks) =
            self.chunks_to_execute(min_traces, &secn_instances);

        let ordered_chunks = self.order_chunks(&chunks_to_execute, &global_id_chunks);
        let global_ids: Vec<usize> = secn_instances.keys().copied().collect();

        let collect_start_times: Vec<AtomicCell<Option<Instant>>> =
            global_ids.iter().map(|_| AtomicCell::new(None)).collect();

        let global_ids_map: HashMap<usize, usize> =
            global_ids.iter().enumerate().map(|(idx, &id)| (id, idx)).collect();

        // Create data buses for each chunk
        let data_buses = self
            .sm_bundle
            .build_data_bus_collectors(&pctx, &secn_instances, &chunks_to_execute)
            .into_iter()
            .map(Mutex::new)
            .collect::<Vec<_>>();

        let n_chunks_left: Vec<AtomicUsize> = global_ids
            .iter()
            .map(|global_id| AtomicUsize::new(global_id_chunks[global_id].len()))
            .collect();

        for global_id in global_ids.iter() {
            let (airgroup_id, air_id) =
                pctx.dctx_get_instance_info(*global_id).expect("Failed to get instance info");
            let stats = Stats {
                airgroup_id,
                air_id,
                collect_start_time: Instant::now(),
                collect_duration: 0,
                witness_start_time: Instant::now(),
                witness_duration: 0,
                num_chunks: global_id_chunks[global_id].len(),
            };

            self.collectors_by_instance
                .write()
                .unwrap()
                .insert(*global_id, (0..global_id_chunks[global_id].len()).map(|_| None).collect());
            self.stats.insert_witness_stats(*global_id, stats);
        }

        let next_chunk = AtomicUsize::new(0);

        rayon::in_place_scope(|scope| {
            for _ in 0..rayon::current_num_threads() {
                let next_chunk = &next_chunk;
                let n_chunks_left = &n_chunks_left;
                let collectors_by_instance = &self.collectors_by_instance;
                let collect_start_times = &collect_start_times;
                let _stats = &self.stats;
                let min_traces = &min_traces;
                let data_buses = &data_buses;
                let zisk_rom = &self.zisk_rom;
                let global_ids_map = &global_ids_map;
                let global_id_chunks = &global_id_chunks;
                let ordered_chunks = &ordered_chunks;
                let chunks_to_execute = &chunks_to_execute;
                let pctx = &pctx;

                scope.spawn(move |_| loop {
                    let next_chunk_id = next_chunk.fetch_add(1, Ordering::Relaxed);
                    if next_chunk_id >= ordered_chunks.len() {
                        break;
                    }
                    let chunk_id = ordered_chunks[next_chunk_id];

                    if let Some(mut data_bus) = data_buses[chunk_id].lock().unwrap().take() {
                        for global_id in chunks_to_execute[chunk_id].iter() {
                            let start_time_cell = &collect_start_times[global_ids_map[global_id]];
                            if start_time_cell.load().is_none() {
                                start_time_cell.store(Some(Instant::now()));
                            }
                        }

                        ZiskEmulator::process_emu_traces::<F, _, _>(
                            zisk_rom,
                            min_traces,
                            chunk_id,
                            &mut data_bus,
                        );

                        // Collect all device results locally to minimize lock acquisitions
                        let devices = data_bus.into_devices(false);
                        let mut entries: Vec<(usize, usize, Option<ChunkCollector>)> = Vec::new();
                        let mut affected_globals: Vec<(usize, usize)> = Vec::new();

                        for (global_id, collector) in devices {
                            if let Some(global_id) = global_id {
                                let global_id_idx = *global_ids_map
                                    .get(&global_id)
                                    .expect("Global ID not found in map");

                                let chunk_order = &global_id_chunks[&global_id];
                                let position = chunk_order
                                    .iter()
                                    .position(|&id| id == chunk_id)
                                    .expect("Chunk ID not found in order");

                                entries.push((
                                    global_id,
                                    position,
                                    Some((chunk_id, collector.unwrap())),
                                ));
                                affected_globals.push((global_id, global_id_idx));
                            }
                        }

                        // Single write-lock acquisition to flush all results from this chunk
                        {
                            let mut guard = collectors_by_instance.write().unwrap();
                            for (global_id, position, entry) in entries.iter_mut() {
                                guard.get_mut(global_id).unwrap()[*position] = entry.take();
                            }
                        }

                        // Update atomic counters and mark ready instances (no lock needed)
                        for (global_id, global_id_idx) in affected_globals {
                            if n_chunks_left[global_id_idx].fetch_sub(1, Ordering::SeqCst) == 1 {
                                pctx.set_witness_ready(global_id, true);

                                let collect_start_time = collect_start_times[global_id_idx]
                                    .load()
                                    .expect("Collect start time was not set");
                                let collect_duration =
                                    collect_start_time.elapsed().as_millis() as u64;

                                let (airgroup_id, air_id) = pctx
                                    .dctx_get_instance_info(global_id)
                                    .expect("Failed to get instance info");
                                let stats = Stats {
                                    airgroup_id,
                                    air_id,
                                    collect_start_time,
                                    collect_duration,
                                    witness_start_time: Instant::now(),
                                    witness_duration: 0,
                                    num_chunks: global_id_chunks[&global_id].len(),
                                };

                                _stats.insert_witness_stats(global_id, stats);
                            }
                        }
                    }
                });
            }
        });
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
    ) -> ProofmanResult<()> {
        #[cfg(feature = "stats")]
        let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id)?;
        stats_begin!(self.stats, _caller_stats_id, _stats_scope, "AIR_WITNESS_TABLE", air_id);

        assert_eq!(table_instance.instance_type(), InstanceType::Table, "Instance is not a table");

        if let Some(air_instance) =
            table_instance.compute_witness(pctx, sctx, vec![], trace_buffer)?
        {
            if pctx
                .dctx_is_my_process_instance(global_id)
                .expect("Failed to check instance ownership")
            {
                pctx.add_air_instance(air_instance, global_id);
            }
        }

        stats_end!(self.stats, &_stats_scope);

        Ok(())
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
        *self.min_traces.write().unwrap() = None;
        *self.secn_planning.write().unwrap() = Vec::new();
        self.main_instances.write().unwrap().clear();
        self.secn_instances.write().unwrap().clear();
        self.collectors_by_instance.write().unwrap().clear();
        self.stats.reset();
    }
}

impl<F: PrimeField64> WitnessComponent<F> for ZiskExecutor<F> {
    /// Executes the ZisK ROM program and calculate the plans for main and secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    ///
    /// # Returns
    /// A vector of global IDs for the instances to compute witness for.
    fn execute(
        &self,
        pctx: Arc<ProofCtx<F>>,
        global_ids: &RwLock<Vec<usize>>,
    ) -> ProofmanResult<()> {
        self.reset();

        stats_begin!(self.stats, 0, _exec_scope, "EXECUTE", 0);

        // Set the start time of the current execution
        self.stats.set_start_time(Instant::now());

        // Process and write precompile atomically
        if let Ok(mut hints_stream_guard) = self.hints_stream.lock() {
            if let Some(hints_stream) = hints_stream_guard.as_mut() {
                let _ = hints_stream.start_stream();
            }
        }

        // Process the ROM to collect the Minimal Traces
        timer_start_info!(COMPUTE_MINIMAL_TRACE);

        let (min_traces, main_count, mut secn_count, handle_mo, execution_result) =
            self.emulator.execute(&self.stdin, &pctx, &self.sm_bundle, &self.stats, &_exec_scope);

        timer_stop_and_log_info!(COMPUTE_MINIMAL_TRACE);

        // Store the execution result
        *self.execution_result.lock().unwrap() = execution_result;

        // Plan the main and secondary instances using the counted metrics
        stats_begin!(self.stats, &_exec_scope, _main_plan_scope, "MAIN_PLAN", 0);

        timer_start_info!(PLAN);
        let (main_planning, public_values) =
            MainPlanner::plan::<F>(&min_traces, main_count, self.chunk_size);
        *self.min_traces.write().unwrap() = Some(min_traces);
        self.assign_main_instances(&pctx, global_ids, main_planning);

        stats_end!(self.stats, &_main_plan_scope);
        stats_begin!(self.stats, &_exec_scope, _secn_plan_scope, "SECN_PLAN", 0);

        let mut secn_planning = self.sm_bundle.plan_sec(&mut secn_count);

        timer_stop_and_log_info!(PLAN);

        timer_start_info!(PLAN_MEM_CPP);
        stats_end!(self.stats, &_secn_plan_scope);

        if let Some(handle_mo) = handle_mo {
            stats_begin!(self.stats, &_exec_scope, _mo_wait_scope, "MO_PLAN_WAIT", 0);

            // Wait for the memory operations thread to finish
            let asm_runner_mo =
                handle_mo.join().expect("Error during Assembly Memory Operations thread execution");

            stats_end!(self.stats, &_mo_wait_scope);
            stats_begin!(self.stats, &_exec_scope, _mo_add_scope, "MO_PLAN_ADD", 0);

            secn_planning
                .entry(self.sm_bundle.get_mem_sm_id())
                .or_default()
                .extend(asm_runner_mo.plans);

            stats_end!(self.stats, &_mo_add_scope);
        }

        timer_stop_and_log_info!(PLAN_MEM_CPP);

        stats_begin!(self.stats, &_exec_scope, _config_scope, "CONFIGURE_INSTANCES", 0);

        // Configure the instances
        self.sm_bundle.configure_instances(&pctx, &secn_planning);

        // Flatten all plans
        let mut secn_planning =
            secn_planning.into_iter().flat_map(|(_, plans)| plans).collect::<Vec<_>>();

        // Assign the instances
        self.assign_secn_instances(&pctx, global_ids, &mut secn_planning);

        // Get the global IDs of the instances to compute witness for
        let secn_global_ids =
            secn_planning.iter().map(|plan| plan.global_id.unwrap()).collect::<Vec<_>>();
        let secn_global_ids_vec: Vec<usize> = secn_global_ids.to_vec();

        // Add public values to the proof context
        let mut publics = ZiskPublicValues::from_vec_guard(pctx.get_publics());
        for (index, value) in public_values.iter() {
            publics.inputs[*index as usize] = F::from_u32(*value);
        }
        drop(publics);

        // Update internal state with the computed minimal traces and planning.
        *self.secn_planning.write().unwrap() = secn_planning;

        let mut secn_instances = self.secn_instances.write().unwrap();
        for global_id in &secn_global_ids_vec {
            secn_instances
                .entry(*global_id)
                .or_insert_with(|| self.create_secn_instance(*global_id));
            secn_instances[global_id].reset();
            if secn_instances[global_id].instance_type() == InstanceType::Instance {
                let checkpoint = secn_instances[global_id].check_point();
                let chunks = match checkpoint {
                    CheckPoint::None => vec![],
                    CheckPoint::Single(chunk_id) => vec![chunk_id.as_usize()],
                    CheckPoint::Multiple(chunk_ids) => {
                        chunk_ids.iter().map(|id| id.as_usize()).collect()
                    }
                };
                let (_, air_id) =
                    pctx.dctx_get_instance_info(*global_id).expect("Failed to get instance info");
                let mem_global_id = air_id == MEM_AIR_IDS[0]
                    || air_id == ROM_DATA_AIR_IDS[0]
                    || air_id == INPUT_DATA_AIR_IDS[0];
                pctx.dctx_set_chunks(*global_id, chunks, mem_global_id);
            }
        }

        if let Ok(mut hints_stream_guard) = self.hints_stream.lock() {
            if let Some(hints_stream) = hints_stream_guard.as_mut() {
                hints_stream.reset();
            }
        }

        stats_end!(self.stats, &_config_scope);
        stats_end!(self.stats, &_exec_scope);

        // #[cfg(feature = "stats")]
        // self.stats.store_stats();

        Ok(())
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
    ) -> ProofmanResult<()> {
        if stage != 1 {
            return Ok(());
        }

        stats_begin!(self.stats, 0, _witness_scope, "CALCULATE_WITNESS", 0);

        let is_asm_emulator = self.emulator.is_asm_emulator();

        let pool = create_pool(n_cores);
        pool.install(|| -> ProofmanResult<()> {
            for &global_id in global_ids {
                let (airgroup_id, air_id) =
                    pctx.dctx_get_instance_info(global_id).expect("Failed to get instance info");

                if MAIN_AIR_IDS.contains(&air_id) {
                    let main_instance = &self.main_instances.read().unwrap()[&global_id];

                    self.witness_main_instance(
                        &pctx,
                        main_instance,
                        buffer_pool.take_buffer(),
                        _witness_scope.id(),
                    )?;
                } else {
                    let secn_instance = &self.secn_instances.read().unwrap()[&global_id];

                    match secn_instance.instance_type() {
                        InstanceType::Instance => {
                            if !self.collectors_by_instance.read().unwrap().contains_key(&global_id)
                            {
                                if air_id == ROM_AIR_IDS[0] && is_asm_emulator {
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
                                    self.stats.insert_witness_stats(global_id, stats);
                                } else {
                                    let mut secn_instances = HashMap::new();
                                    secn_instances.insert(global_id, secn_instance);
                                    self.witness_collect_instances(pctx.clone(), secn_instances);
                                }
                            }
                            self.witness_secn_instance(
                                &pctx,
                                &sctx,
                                global_id,
                                &**secn_instance,
                                buffer_pool.take_buffer(),
                                _witness_scope.id(),
                            )?;
                        }
                        InstanceType::Table => self.witness_table(
                            &pctx,
                            &sctx,
                            global_id,
                            &**secn_instance,
                            Vec::new(),
                            _witness_scope.id(),
                        )?,
                    }
                }
            }
            Ok(())
        })?;
        stats_end!(self.stats, &_witness_scope);

        Ok(())
    }

    fn pre_calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        _sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        _buffer_pool: &dyn BufferPool<F>,
    ) -> ProofmanResult<()> {
        stats_begin!(self.stats, 0, _pre_scope, "PRE_CALCULATE_WITNESS", 0);

        if stage != 1 {
            return Ok(());
        }
        let secn_instances_guard = self.secn_instances.read().unwrap();

        let is_asm_emulator = self.emulator.is_asm_emulator();

        let mut secn_instances = HashMap::new();
        for &global_id in global_ids {
            let (airgroup_id, air_id) =
                pctx.dctx_get_instance_info(global_id).expect("Failed to get instance info");
            if MAIN_AIR_IDS.contains(&air_id) {
                pctx.set_witness_ready(global_id, false);
            } else if air_id == ROM_AIR_IDS[0] {
                if is_asm_emulator {
                    pctx.set_witness_ready(global_id, false);
                } else {
                    let secn_instance = &secn_instances_guard[&global_id];
                    let rom_instance =
                        secn_instance.as_any().downcast_ref::<RomInstance>().unwrap();
                    if rom_instance.skip_collector() {
                        let stats = Stats {
                            airgroup_id,
                            air_id,
                            collect_start_time: Instant::now(),
                            collect_duration: 0,
                            witness_start_time: Instant::now(),
                            witness_duration: 0,
                            num_chunks: 0,
                        };

                        self.collectors_by_instance.write().unwrap().insert(global_id, Vec::new());
                        self.stats.insert_witness_stats(global_id, stats);
                        pctx.set_witness_ready(global_id, true);
                    } else {
                        secn_instances.insert(global_id, secn_instance);
                    }
                }
            } else {
                let secn_instance = &secn_instances_guard[&global_id];

                if secn_instance.instance_type() == InstanceType::Instance
                    && !self.collectors_by_instance.read().unwrap().contains_key(&global_id)
                {
                    secn_instances.insert(global_id, secn_instance);
                } else {
                    pctx.set_witness_ready(global_id, true);
                }
            }
        }

        let pool = create_pool(n_cores);
        pool.install(|| {
            if !secn_instances.is_empty() {
                self.witness_collect_instances(pctx.clone(), secn_instances);
            }
        });

        stats_end!(self.stats, &_pre_scope);
        Ok(())
    }

    /// Debugs the main and secondary state machines.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_ids` - Global IDs of the instances to debug.
    fn debug(
        &self,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
    ) -> ProofmanResult<()> {
        for &global_id in global_ids {
            let (_airgroup_id, air_id) =
                pctx.dctx_get_instance_info(global_id).expect("Failed to get instance info");

            if MAIN_AIR_IDS.contains(&air_id) {
                MainSM::debug(&pctx, &sctx);
            } else {
                let secn_instances = self.secn_instances.read().unwrap();
                let secn_instance = secn_instances.get(&global_id).expect("Instance not found");

                secn_instance.debug(&pctx, &sctx);
            }
        }
        Ok(())
    }
}
