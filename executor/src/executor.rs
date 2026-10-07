//! The `ZiskExecutor` module serves as the core orchestrator for executing the ZisK ROM program
//! and generating witness computations.
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

use crate::{
    plan::MemPlanArtifacts, ports::GlobalId, ports::ProofRegistry, sm::MEM_POSITION,
    witness::WitnessContext, AirClassifier, AsmResources, EmulatorAsm, ExecutionPhase,
    ExecutionState, InstanceAssigner, NoopProofRegistry, PlanPhase, ProofmanAdapter,
    StaticSMBundle, WitnessPhase,
};
use proofman_common::{lease_pool, BufferPool, ProofCtx, ProofmanError, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use proofman_witness::{WitnessComponent, WitnessManager};
use zisk_asm_runner::OwnedMemInstances;
use zisk_pil::{INPUT_DATA_AIR_IDS, MEM_AIR_IDS, ROM_DATA_AIR_IDS};

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Instant,
};
use zisk_common::{
    io::ZiskStdin, stats_begin, stats_end, AirInstanceCount, BusDeviceMetrics, ChunkId, EmuTrace,
    ExecutorStatsHandle, Plan, ZiskExecutorSummary, ZiskExecutorTime,
};
use zisk_core::{ZiskRom, CHUNK_SIZE};
use zisk_sm_main::{MainPlanner, MainSM};

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};

/// `(chunk_id, metrics)` pair — the per-chunk device-metrics output
/// produced by counter-phase processing.
pub(crate) type DeviceMetricsByChunk = (ChunkId, Box<dyn BusDeviceMetrics>);

/// One entry in the standalone plan summary — counts of planned instances
/// per AIR. No proving-key / setup data; just shape from the planner.
pub struct PlanSummaryEntry {
    /// AIR group id.
    pub airgroup_id: usize,
    /// AIR id within the group.
    pub air_id: usize,
    /// Display name for this AIR (e.g. "Main", "Mem", "Keccakf"). "Unknown" for unregistered ids.
    pub name: &'static str,
    /// Number of instances planned for this AIR.
    pub count: usize,
}

/// The maximum number of steps to execute in the emulator or assembly runner.
pub(crate) const MAX_NUM_STEPS: u64 = 1 << 36;

/// Appends to the progressive minimal-trace store every chunk it has not seen
/// yet, up to and including `idx`. `traces` is the emulator's own buffer, so
/// `traces[idx]` is its last element.
///
/// In-order delivery is an invariant of the ASM reader; a chunk that does not
/// extend the store contiguously is rejected rather than silently skipped.
fn publish_chunks(
    store: &mut Vec<Arc<EmuTrace>>,
    traces: &[Arc<EmuTrace>],
    idx: usize,
) -> ExecutorResult<()> {
    if store.len() > idx {
        return Err(ExecutorError::ChunkOutOfOrder { got: idx, expected: store.len() });
    }
    store.extend_from_slice(&traces[store.len()..=idx]);
    Ok(())
}

/// The `ZiskExecutor` struct orchestrates the execution of the ZisK ROM program, managing state
/// machines, planning, and witness computation.
pub struct ZiskExecutor<F: PrimeField64> {
    /// Shared execution state.
    state: ExecutionState<F>,
    /// Phase-1 Execution. Runs the chosen emulator and produces an `ExecutionOutput`.
    execution: ExecutionPhase,
    /// Phase-2 Plan (pure planning, no bundle).
    plan: PlanPhase<F>,
    /// Phase-3 Witness computation. `None` on the standalone path
    /// (executor constructed without `WitnessManager` / `Std`).
    witness: Option<WitnessPhase<F>>,
    /// Whether the FROPS multiplicity column should come from the ROM-histogram assembly when
    /// this executor runs on the ASM path. Only a request: it is applied at the start of every
    /// execution, together with the backend that execution actually uses. See
    /// [`Self::set_frops_multiplicity_from_asm`].
    frops_from_asm_requested: AtomicBool,
}

impl<F: PrimeField64> ZiskExecutor<F> {
    /// Creates a new instance of the `ZiskExecutor` with default state machines.
    ///
    /// This function initializes the executor with a default set of state machines.
    ///
    /// # Arguments
    ///
    /// * `wcm` - Witness manager for managing witness data.
    /// * `verbose_mode` - Verbose mode for logging.
    /// * `shared_tables` - Whether to use shared tables for execution.
    /// * `with_asm_emulator` - Whether the executor supports the ASM backend at runtime.
    /// * `packed` - Whether to use packed representation for witness computation.
    pub fn new(
        wcm: &WitnessManager<F>,
        verbose_mode: proofman_common::VerboseMode,
        shared_tables: bool,
        with_asm_emulator: bool,
        packed: bool,
    ) -> ExecutorResult<Arc<Self>> {
        let rank_info = wcm.get_rank_info();
        proofman_common::initialize_logger(verbose_mode, Some(&rank_info));

        let std = pil2_std_lib::Std::new(wcm.get_pctx(), wcm.get_sctx(), shared_tables)?;
        proofman::register_std(wcm, &std);

        let precompiles = crate::Precompiles::all();
        let sm_bundle = Arc::new(StaticSMBundle::new(std, precompiles));

        let executor = Arc::new(Self {
            state: ExecutionState::new(),
            execution: ExecutionPhase::new(CHUNK_SIZE, with_asm_emulator),
            plan: PlanPhase::new(CHUNK_SIZE),
            witness: Some(WitnessPhase::new(CHUNK_SIZE, sm_bundle)),
            frops_from_asm_requested: AtomicBool::new(false),
        });
        executor.set_packed(packed);

        wcm.register_component(executor.clone());
        wcm.set_witness_initialized();

        Ok(executor)
    }

    /// Constructs a standalone executor — no `WitnessManager`, no `Std`,
    /// no `StaticSMBundle`, no `WitnessPhase`. Only the emulate + plan
    /// path is wired up; calls to witness-mode-only public methods (e.g.
    /// `calculate_witness`) will panic.
    pub fn new_standalone(
        verbose_mode: proofman_common::VerboseMode,
        with_asm_emulator: bool,
    ) -> ExecutorResult<Arc<Self>> {
        proofman_common::initialize_logger(verbose_mode, None);
        Ok(Arc::new(Self {
            state: ExecutionState::new(),
            execution: ExecutionPhase::new(CHUNK_SIZE, with_asm_emulator),
            plan: PlanPhase::new(CHUNK_SIZE),
            witness: None,
            frops_from_asm_requested: AtomicBool::new(false),
        }))
    }

    /// Standalone execution entry point: emulate + count + plan. Returns
    /// the executor summary, the program's captured `(index, value)`
    /// public-output pairs, and a per-AIR plan summary. `cost_per_type`
    /// on the returned summary is `Default::default()` since cost
    /// computation is skipped (no `SetupCtx`).
    #[allow(clippy::type_complexity)]
    pub fn execute_standalone(
        &self,
        zisk_rom: Arc<ZiskRom>,
        stdin: ZiskStdin,
        use_hints: bool,
    ) -> ExecutorResult<(ZiskExecutorSummary, Vec<(u64, u32)>, Vec<PlanSummaryEntry>)> {
        self.state.set_rom(zisk_rom, use_hints);
        self.state.set_stdin(stdin);
        let registry = NoopProofRegistry::default();
        let global_ids = RwLock::new(Vec::new());
        self.execute_inner(&registry, None, &global_ids)?;

        let mut plan: Vec<PlanSummaryEntry> = registry
            .take_instance_counts()
            .into_iter()
            .map(|((airgroup_id, air_id), count)| PlanSummaryEntry {
                airgroup_id,
                air_id,
                name: AirClassifier::name(airgroup_id, air_id),
                count,
            })
            .collect();
        plan.sort_by_key(|e| (e.airgroup_id, e.air_id));

        Ok((self.state.get_execution_result(), registry.take_pub_outs(), plan))
    }

    /// Sets the ZisK ROM (ELF) for execution.
    ///
    /// This method allows changing the ROM between executions without
    /// recreating the executor, making the executor more reusable.
    ///
    /// # Arguments
    /// * `zisk_rom` - The ZisK ROM to execute.
    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>, use_hints: bool) -> ExecutorResult<()> {
        self.state.set_rom(zisk_rom.clone(), use_hints);
        if let Some(witness) = self.witness.as_ref() {
            witness.set_rom(zisk_rom)?;
        }
        Ok(())
    }

    /// Selects where the FROPS multiplicity column comes from on the ASM path.
    ///
    /// With `true` it is taken from the column the ROM-histogram assembly builds, which a single
    /// worker computes and hands over with the ROM histogram; the state-machine collectors must then
    /// not accumulate it as well. With `false` (the default) the collectors own it. See
    /// `executor::sm::frops`.
    ///
    /// This is a request, not the setting itself: the same executor can switch between the ASM and
    /// the Rust backend from one job to the next (`set_asm_resources` / `clear_asm_resources`), and
    /// the Rust path has no ROM-histogram assembly to take the column from. So every execution
    /// applies it as `requested && ASM path`, just before it runs.
    pub fn set_frops_multiplicity_from_asm(&self, from_asm: bool) {
        self.frops_from_asm_requested.store(from_asm, Ordering::Relaxed);
    }

    /// Sets whether to use packed representation for witness computation.
    pub fn set_packed(&self, packed: bool) {
        if let Some(witness) = self.witness.as_ref() {
            witness.set_packed(packed);
        }
    }

    /// Whether traces are built bit-packed.
    pub fn is_packed(&self) -> bool {
        self.witness.as_ref().map(|w| w.is_packed()).unwrap_or(false)
    }

    /// Sets the standard input for execution.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> ExecutorResult<()> {
        self.state.set_stdin(stdin);
        Ok(())
    }

    /// Sets ASM resources for execution (only applicable for ASM emulator).
    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> ExecutorResult<()> {
        self.execution.set_asm_resources(asm_resources)
    }

    /// Clears any previously-installed ASM resources. No-op when the
    /// executor was built with the Rust emulator backend.
    pub fn clear_asm_resources(&self) -> ExecutorResult<()> {
        self.execution.clear_asm_resources();
        Ok(())
    }

    /// Retires whatever the previous job left behind: drains a ROM-histogram runner
    /// nobody consumed, then clears the hints stream and the input shmem.
    ///
    /// This is the job boundary. Call it after the previous computation is idle and
    /// **before** the next job's inputs or hints are written, since the reset rewinds
    /// the input shmem and marks the hints stream uninitialised. Drain first: the reset
    /// drains the semaphores the RH child is waiting on, so rewinding while that child
    /// is still reading strands it, and the next job's runner then hangs behind it.
    ///
    /// Blocks if the runner is still going, which is why it must not run before
    /// cancellation has been signalled on a job that failed or was cancelled.
    pub fn reset_for_new_job(&self) -> ExecutorResult<()> {
        self.drain_rh();
        self.execution.reset()
    }

    /// Joins a ROM-histogram runner left unconsumed by a previous execution and releases
    /// its histogram. Idempotent, and a no-op when nothing is parked.
    fn drain_rh(&self) {
        if let Some(witness) = self.witness.as_ref() {
            witness.drain_rh();
        }
    }

    /// Returns a reference to the ASM emulator if ASM execution is active.
    pub fn asm_emulator(&self) -> Option<&EmulatorAsm> {
        self.execution.asm_emulator()
    }

    /// Gets the execution result and stats.
    #[allow(clippy::type_complexity)]
    pub fn get_execution_result(&self) -> (ZiskExecutorSummary, ExecutorStatsHandle) {
        (self.state.get_execution_result(), self.state.get_stats())
    }

    /// Stores statistics to persistent storage.
    pub fn store_stats(&self) {
        self.state.stats.store_stats();
    }

    /// Inner implementation of [`WitnessComponent::execute`].
    ///
    /// Returns [`ExecutorResult`] so the body can use `?` freely; the
    /// trait-method wrapper maps any error to `ProofmanError::InvalidSetup`
    /// once at the FFI seam.
    fn execute_inner(
        &self,
        registry: &dyn ProofRegistry,
        proofman_extras: Option<&ProofmanAdapter<'_, F>>,
        global_ids: &RwLock<Vec<usize>>,
    ) -> ExecutorResult<()> {
        let start_total = Instant::now();

        // Debug cross-check (`debug_frops` feature): by now the previous execution has collected
        // every instance, so this is the first point at which rows the assembly counted and no
        // collector claimed can be told apart from rows not yet reached. Nothing is armed unless the
        // variable is set, so this is a no-op in a normal run.
        if let Err(problem) = zisk_core::frops::frops_check_report(true) {
            tracing::error!("FROPS cross-check of the previous execution: {problem}");
        }

        self.state.reset();
        if let Some(witness) = self.witness.as_ref() {
            witness.reset()?;
        }

        stats_begin!(self.state.stats, 0, _exec_scope, "EXECUTE", 0);
        self.state.stats.set_start_time(Instant::now());

        let is_asm_emulator = self.execution.is_asm_execution();

        // Decide the FROPS multiplicity producer for this execution, from the backend it actually
        // runs on. Every reader comes later: the collectors built after this execution, and the ROM
        // witness, which publishes the column (`publish_frops_from_asm`).
        if let Some(witness) = self.witness.as_ref() {
            let from_asm = is_asm_emulator && self.frops_from_asm_requested.load(Ordering::Relaxed);
            witness.set_frops_multiplicity_from_asm(from_asm);
        }

        // Reserve proofman's unified GPU buffer for MO count-and-plan
        // (no-op on CPU / standalone).
        if is_asm_emulator {
            if let Some(extras) = proofman_extras {
                extras.acquire_gpu_buffer();
            }
        }

        // ────────────────────────────────────────────────────────────
        // Phase 1.1: Emulate (+ incremental Main advancement)
        // ────────────────────────────────────────────────────────────
        // ROM instance is assigned BEFORE the run so the global-id sequence
        // (ROM, Main segments in order, secondary) is identical to the old
        // batch path while Main segments are now assigned mid-emulation.
        InstanceAssigner::assign_rom_instance(registry)?;

        // Incremental Main-instance advancement. The ASM MT reader calls this on
        // this thread, in chunk order: as soon as a chunk completes a Main
        // instance — or ends the execution — that instance's chunks are published
        // to `state.min_traces` and the instance is planned, assigned and marked
        // witness-ready, so main witness computation (and its GPU streaming-slot
        // commit) overlaps the rest of the emulation instead of waiting for the
        // run to finish. Global-id order is unchanged: ROM (assigned above), then
        // Main segments in order, then secondary.
        //
        // No-op in standalone mode and never called by the Rust emulator; both
        // keep the post-run batch path below.
        // Two rounds, with the first collected early and every secondary placed on registration,
        // only when the memory airs come from the device: with the CPU witness the memory
        // instances would need a second collect pass of their own, which costs more than the
        // overlap gains (641_54, 24 cores: +0.7 s), so that path keeps the one-round flow.
        let two_rounds = is_asm_emulator && zisk_asm_runner::device_mem_witness_requested();
        // `ZISK_EARLY_SECN=1`: the precompile instances the counted chunks already fill are
        // registered and collected while the emulation runs (two-round mode only).
        let early_secn = two_rounds
            && self.witness.is_some()
            && proofman_extras.is_some()
            && std::env::var("ZISK_EARLY_SECN").as_deref() == Ok("1");

        std::thread::scope(|early_scope| -> ExecutorResult<()> {
            // Per precompile position, the (air, chunks) of each instance registered early, in order.
            let early_done: Mutex<BTreeMap<usize, Vec<(usize, Vec<usize>)>>> =
                Mutex::new(BTreeMap::new());
            let early_collects = Mutex::new(Vec::new());
            let precompile_positions: Vec<usize> =
                if early_secn { crate::sm::precompile_positions().collect() } else { Vec::new() };

            let num_within = MainPlanner::traces_per_segment(self.plan.chunk_size())?;
            let on_chunk = |idx: usize,
                            traces: &[Arc<EmuTrace>],
                            is_last: bool,
                            counted: &dyn crate::CountedPrefix|
             -> ExecutorResult<()> {
                let Some(witness) = self.witness.as_ref() else { return Ok(()) };

                // This chunk neither completes a Main instance nor ends the execution.
                let Some(segment) = MainPlanner::segment_completed_by(idx, num_within, is_last)
                else {
                    return Ok(());
                };

                {
                    let mut guard = self.state.min_traces.write_or_poison("min_traces")?;
                    publish_chunks(guard.get_or_insert_with(Vec::new), traces, idx)?;
                }

                let plan = MainPlanner::plan_segment(segment, is_last);
                let assignments =
                    InstanceAssigner::assign_main_instances(registry, global_ids, vec![plan])?;
                witness.populate_main_instances(registry, &self.state, assignments)?;

                // The precompile instances the chunks counted so far fill completely: placed in
                // order now, and collected on their own thread while the emulation goes on.
                let Some(extras) = proofman_extras.filter(|_| early_secn && !is_last) else {
                    return Ok(());
                };
                let mut planning: BTreeMap<usize, Vec<Plan>> = BTreeMap::new();
                {
                    let mut done = early_done.lock().unwrap_or_else(|e| e.into_inner());
                    for &position in &precompile_positions {
                        let registered = done.get(&position).map_or(0, Vec::len);
                        let mut fresh = Vec::new();
                        counted.visit(position, &mut |prefix| {
                            fresh =
                                crate::sm::plan_sec_prefix::<F>(position, is_asm_emulator, prefix)
                                    .into_iter()
                                    .skip(registered)
                                    .collect();
                        });
                        if !fresh.is_empty() {
                            done.entry(position).or_default().extend(
                                fresh
                                    .iter()
                                    .map(|p: &Plan| (p.air_id, checkpoint_chunks(&p.check_point))),
                            );
                            planning.insert(position, fresh);
                        }
                    }
                }
                if planning.is_empty() {
                    return Ok(());
                }
                let (plans, ids) =
                    self.assign_secn_round(registry, global_ids, proofman_extras, planning, true)?;
                tracing::debug!("early secondaries at chunk {idx}: {} instances", ids.len());
                self.populate_secn_round(registry, plans, &ids)?;
                let handle = early_scope.spawn(move || {
                    witness.pre_calculate(extras.pctx(), extras, &self.state, &ids, is_asm_emulator)
                });
                early_collects.lock().unwrap_or_else(|e| e.into_inner()).push(handle);
                Ok(())
            };
            let chunk_hook = &on_chunk;

            timer_start_info!(COMPUTE_MINIMAL_TRACE);
            let start_partial = Instant::now();

            let zisk_rom = self.state.get_rom()?;
            let stdin = self.state.get_stdin();
            let output = self.execution.run::<F>(
                &zisk_rom,
                &stdin,
                // The ROM-histogram runner is what asks the assembly child for a histogram,
                // so not spawning it leaves that child idle. Skipped without a witness:
                // there is no ROM state machine to read one, and the standalone path would
                // pay for a full histogram pass only to drop the result.
                self.witness.is_some() && registry.is_first_process(),
                self.state.use_hints.load(std::sync::atomic::Ordering::SeqCst),
                &self.state.stats,
                &_exec_scope,
                chunk_hook,
            )?;

            let execution_duration = start_partial.elapsed();
            timer_stop_and_log_info!(COMPUTE_MINIMAL_TRACE);

            // ────────────────────────────────────────────────────────────
            // Phase 1.2: Plan + assign main, then populate main (witness only)
            // ────────────────────────────────────────────────────────────
            let steps = output.steps;

            let crate::ExecutionOutput { min_traces, mut counters, pub_outs, mut backend, .. } =
                output;
            let num_chunks = min_traces.len();

            // Hand the ROM-histogram runner over without joining it: the runner outlives the
            // minimal-trace run, and the instance it feeds does not compute its witness until
            // much later, so the join belongs there. Parking also selects that instance's ASM
            // backend, so it must precede `populate_secn_instances` below.
            //
            // Parked here rather than after the planning phases so that an error in between
            // still leaves the handle where the next job's drain can find it — otherwise its
            // child could still be consuming input shmem when that job resets it.
            if let (Some(handle), Some(witness)) = (backend.take_rh_handle(), self.witness.as_ref())
            {
                witness.park_rh_handle(handle)?;
            }

            // The hook published every chunk it saw, so on the ASM path the store is
            // already complete and a read lock is enough (an exclusive lock here would
            // stall main witnesses that are already computing). The Rust emulator
            // never calls the hook, so its store is still empty and gets the whole
            // vector at once.
            let published =
                self.state.min_traces.read_or_poison("min_traces")?.as_ref().map_or(0, Vec::len);
            if published != num_chunks {
                *self.state.min_traces.write_or_poison("min_traces")? = Some(min_traces);
            }

            // ASM + witness: the hook already released every Main instance during the
            // run (the runner only returns `Ok` after delivering the final chunk).
            let main_instances_count = if is_asm_emulator && self.witness.is_some() {
                num_chunks.div_ceil(num_within)
            } else {
                // Rust emulator / standalone: plan and release every segment now.
                let main_plans = self.plan.run_main(num_chunks, &self.state.stats, &_exec_scope)?;
                let main_assignments =
                    InstanceAssigner::assign_main_instances(registry, global_ids, main_plans)?;
                let count = main_assignments.len();
                if let Some(witness) = self.witness.as_ref() {
                    witness.populate_main_instances(registry, &self.state, main_assignments)?;
                }
                count
            };

            // ────────────────────────────────────────────────────────────
            // Phase 1.3: Plan and register the secondaries in two rounds (witness only)
            // ────────────────────────────────────────────────────────────
            // The minimal trace determines every secondary but the memory ones. Those are
            // registered first and their inputs collected while the memory-ops runner finishes,
            // so their witnesses do not wait for it; the memory plans form the second round.
            let (mut secn_planning, count_and_plan_duration) = self.plan.run_secondary_mt(
                &mut counters,
                num_chunks,
                is_asm_emulator,
                &self.state.stats,
                &_exec_scope,
            )?;
            let mut mem_planning = BTreeMap::new();
            if let Some(mem_mt_plans) = secn_planning.remove(&MEM_POSITION) {
                mem_planning.insert(MEM_POSITION, mem_mt_plans);
            }

            // The instances registered while the emulation ran must open the block's plan of their
            // position unchanged; they are taken out of it so they are not registered twice.
            let early = std::mem::take(&mut *early_done.lock().unwrap_or_else(|e| e.into_inner()));
            for (position, expected) in early {
                let plans = secn_planning.entry(position).or_default();
                if let Some(index) = (0..expected.len()).find(|&i| {
                    plans.get(i).is_none_or(|p| {
                        p.air_id != expected[i].0
                            || checkpoint_chunks(&p.check_point) != expected[i].1
                    })
                }) {
                    return Err(ExecutorError::EarlyPlanMismatch { position, index });
                }
                plans.drain(..expected.len());
            }

            registry.write_pub_outs(&pub_outs.0);

            stats_begin!(self.state.stats, &_exec_scope, _config_scope, "CONFIGURE_INSTANCES", 0);
            let first_round = self.register_secn_round(
                registry,
                global_ids,
                proofman_extras,
                secn_planning,
                two_rounds,
            )?;

            let mem_artifacts = std::thread::scope(|scope| -> ExecutorResult<MemPlanArtifacts> {
                let collecting = match (self.witness.as_ref(), proofman_extras) {
                    (Some(witness), Some(extras)) if two_rounds && !first_round.is_empty() => {
                        Some(scope.spawn(|| {
                            witness.pre_calculate(
                                extras.pctx(),
                                extras,
                                &self.state,
                                &first_round,
                                is_asm_emulator,
                            )
                        }))
                    }
                    _ => None,
                };

                let mut mem_artifacts =
                    self.plan.await_mem_plans(&mut backend, &self.state.stats, &_exec_scope)?;

                // Round 2: the memory plans, registered and placed before the device fills so only
                // the instances this process owns are filled, while the arena is still borrowed.
                let mut mem_plans = std::mem::take(&mut mem_artifacts.mem_plans);
                mem_planning.entry(MEM_POSITION).or_default().append(&mut mem_plans);
                let (plans, ids) = self.assign_secn_round(
                    registry,
                    global_ids,
                    proofman_extras,
                    mem_planning,
                    two_rounds,
                )?;
                let mut release_deferred = false;
                if let Some(device_witness) = mem_artifacts.device_witness.as_ref() {
                    let mut owned = OwnedMemInstances::default();
                    for (plan, &gid) in plans.iter().zip(ids.iter()) {
                        if !registry.is_my_process_instance(GlobalId(gid))? {
                            continue;
                        }
                        let segment = plan.segment_id.map(|s| usize::from(s) as u32);
                        if plan.air_id == MEM_AIR_IDS[0] {
                            owned.ram.extend(segment);
                        } else if plan.air_id == ROM_DATA_AIR_IDS[0] {
                            owned.rom.extend(segment);
                        } else if plan.air_id == INPUT_DATA_AIR_IDS[0] {
                            owned.input.extend(segment);
                        } else if AirClassifier::is_mem_align(plan.air_id) {
                            owned.align.push(plan);
                        }
                    }
                    let d_buffers = proofman_extras
                        .map(|extras| extras.pctx().get_device_buffers_ptr())
                        .unwrap_or(std::ptr::null_mut());
                    release_deferred = device_witness.fill_owned(&owned, d_buffers);
                }

                // MO runner joined and the fills are done; release the buffer back to proofman.
                // Earlier error paths skip the release on purpose: the MO thread may still be using
                // the buffer. In slot mode the release follows the last memory instance's commit.
                if is_asm_emulator {
                    if let Some(extras) = proofman_extras {
                        if let Some(used) = mem_artifacts.gpu_mops_used_bytes {
                            extras.pctx().report_first_gpu_buffer_usage(used);
                        }
                        if !release_deferred {
                            extras.release_gpu_buffer();
                        }
                    }
                }
                self.populate_secn_round(registry, plans, &ids)?;

                if let Some(handle) = collecting {
                    handle
                        .join()
                        .map_err(|_| ExecutorError::SecnPlanMissing { phase: "collect" })??;
                }
                Ok(mem_artifacts)
            })?;

            stats_end!(self.state.stats, &_config_scope);

            // The debug cross-check, which the collectors read when they are built, right after
            // `execute` returns. The multiplicity column itself is published by the ROM witness,
            // which reads the histogram anyway, so the end of execution never waits for it.
            if let Some(witness) = self.witness.as_ref() {
                witness.arm_frops_cross_check()?;
            }

            // ────────────────────────────────────────────────────────────
            // Phase 1.4: Cost accumulation (witness only — needs sctx)
            // ────────────────────────────────────────────────────────────
            let cost_per_type = match proofman_extras {
                Some(extras) => extras.compute_costs(&self.state, main_instances_count)?,
                None => Default::default(),
            };

            stats_end!(self.state.stats, &_exec_scope);

            let zisk_execution_time = ZiskExecutorTime {
                execution_duration: execution_duration.as_millis() as u64,
                count_and_plan_duration: count_and_plan_duration.as_millis() as u64,
                count_and_plan_mo_duration: mem_artifacts.count_and_plan_mo_duration.as_millis()
                    as u64,
                total_duration: start_total.elapsed().as_millis() as u64,
                asm_execution_duration: self.execution.get_asm_execution_info()?,
            };
            let mut execution_result =
                ZiskExecutorSummary::new(steps, zisk_execution_time, cost_per_type);
            // Per-AIR instance plan, captured from the registry's planning counts. Only the
            // full (proofman) path exposes this via the summary; the standalone path returns
            // its own (named) plan directly, so skip the work when there's no `SetupCtx`.
            if proofman_extras.is_some() {
                execution_result.plan = registry
                    .instance_counts()
                    .into_iter()
                    .map(|((airgroup_id, air_id), count)| AirInstanceCount {
                        airgroup_id,
                        air_id,
                        count: count as u64,
                    })
                    .collect();
            }

            // Store the execution result
            self.state.set_execution_result(execution_result);

            for handle in early_collects.into_inner().unwrap_or_else(|e| e.into_inner()) {
                handle
                    .join()
                    .map_err(|_| ExecutorError::SecnPlanMissing { phase: "early collect" })??;
            }
            Ok(())
        })
    }

    /// Registers one round of secondary plans: SM configuration, placement, instances and
    /// checkpoints. Returns the round's global ids.
    fn register_secn_round(
        &self,
        registry: &dyn ProofRegistry,
        global_ids: &RwLock<Vec<usize>>,
        proofman_extras: Option<&ProofmanAdapter<'_, F>>,
        planning: BTreeMap<usize, Vec<Plan>>,
        place_on_registration: bool,
    ) -> ExecutorResult<Vec<usize>> {
        let (plans, ids) = self.assign_secn_round(
            registry,
            global_ids,
            proofman_extras,
            planning,
            place_on_registration,
        )?;
        self.populate_secn_round(registry, plans, &ids)?;
        Ok(ids)
    }

    /// The first half of a round: SM configuration and placement. Returns the plans with their
    /// global ids, so the caller can act on the placement before the instances exist.
    fn assign_secn_round(
        &self,
        registry: &dyn ProofRegistry,
        global_ids: &RwLock<Vec<usize>>,
        proofman_extras: Option<&ProofmanAdapter<'_, F>>,
        planning: BTreeMap<usize, Vec<Plan>>,
        place_on_registration: bool,
    ) -> ExecutorResult<(Vec<Plan>, Vec<usize>)> {
        if let (Some(witness), Some(extras)) = (self.witness.as_ref(), proofman_extras) {
            witness.configure_sm_instances(extras.pctx(), &planning);
        }

        let mut plans: Vec<Plan> = planning.into_values().flatten().collect();
        InstanceAssigner::assign_secn_instances(
            registry,
            global_ids,
            &mut plans,
            place_on_registration,
        )?;
        let ids: Vec<usize> = plans
            .iter()
            .map(|plan| {
                plan.global_id.ok_or(ExecutorError::SecnPlanMissing { phase: "assignment" })
            })
            .collect::<ExecutorResult<Vec<_>>>()?;
        Ok((plans, ids))
    }

    /// The second half of a round: the instances and their checkpoints.
    fn populate_secn_round(
        &self,
        registry: &dyn ProofRegistry,
        plans: Vec<Plan>,
        ids: &[usize],
    ) -> ExecutorResult<()> {
        if let Some(witness) = self.witness.as_ref() {
            witness.populate_secn_instances(&self.state, plans)?;
            witness.configure_checkpoints(registry, &self.state, ids)?;
        }
        Ok(())
    }

    fn witness_or_panic(&self) -> &WitnessPhase<F> {
        self.witness.as_ref().expect("witness phase missing on a witness-mode entry point")
    }

    /// Inner implementation of [`WitnessComponent::calculate_witness`].
    fn calculate_witness_inner(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        buffer_pool: &dyn BufferPool<F>,
    ) -> ExecutorResult<()> {
        if stage != 1 {
            return Ok(());
        }

        stats_begin!(self.state.stats, 0, _witness_scope, "CALCULATE_WITNESS", 0);

        let pool = lease_pool(n_cores);
        let adapter = ProofmanAdapter::new(&pctx, &sctx);
        let is_asm_emulator = self.execution.is_asm_execution();
        let witness = self.witness_or_panic();
        pool.install(|| -> ExecutorResult<()> {
            let ctx = WitnessContext::new(
                &pctx,
                &sctx,
                &self.state,
                buffer_pool,
                &_witness_scope,
                &adapter,
                is_asm_emulator,
            );
            for &global_id in global_ids {
                witness.dispatch(&ctx, global_id)?;
            }
            Ok(())
        })?;

        stats_end!(self.state.stats, &_witness_scope);

        // Debug cross-check: a row claimed beyond what the assembly counted is a disagreement as
        // soon as it happens, so report it here without waiting for the execution to finish. The
        // other direction needs every instance collected; see `execute_inner`.
        if let Err(problem) = zisk_core::frops::frops_check_report(false) {
            tracing::error!("FROPS cross-check: {problem}");
        }

        Ok(())
    }

    /// Inner implementation of [`WitnessComponent::pre_calculate_witness`].
    fn pre_calculate_witness_inner(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        _buffer_pool: &dyn BufferPool<F>,
    ) -> ExecutorResult<()> {
        stats_begin!(self.state.stats, 0, _pre_scope, "PRE_CALCULATE_WITNESS", 0);

        if stage != 1 {
            return Ok(());
        }

        let pool = lease_pool(n_cores);
        let adapter = ProofmanAdapter::new(&pctx, &sctx);
        let is_asm_emulator = self.execution.is_asm_execution();
        let witness = self.witness_or_panic();

        pool.install(|| {
            witness.pre_calculate(&pctx, &adapter, &self.state, global_ids, is_asm_emulator)
        })?;

        stats_end!(self.state.stats, &_pre_scope);
        Ok(())
    }
}

impl<F: PrimeField64> WitnessComponent<F> for ZiskExecutor<F> {
    /// The proof's end: a slot-mode block that still holds the arena hands it back.
    fn end(
        &self,
        _pctx: Arc<ProofCtx<F>>,
        _sctx: Arc<SetupCtx<F>>,
        _debug_info: &proofman_common::DebugInfo,
    ) -> ProofmanResult<()> {
        zisk_asm_runner::device_mem_witness_end();
        Ok(())
    }

    /// Executes the ZisK ROM program and calculate the plans for main and secondary state machines.
    fn execute(
        &self,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &RwLock<Vec<usize>>,
    ) -> ProofmanResult<()> {
        let adapter = ProofmanAdapter::new(&pctx, &sctx);
        self.execute_inner(&adapter, Some(&adapter), global_ids)
            .map_err(|e| ProofmanError::InvalidSetup(format!("{e:#}")))
    }

    /// Computes the witness for the main and secondary state machines.
    fn calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        buffer_pool: &dyn BufferPool<F>,
    ) -> ProofmanResult<()> {
        self.calculate_witness_inner(stage, pctx, sctx, global_ids, n_cores, buffer_pool)
            .map_err(|e| ProofmanError::InvalidSetup(format!("{e:#}")))
    }

    fn pre_calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        buffer_pool: &dyn BufferPool<F>,
    ) -> ProofmanResult<()> {
        self.pre_calculate_witness_inner(stage, pctx, sctx, global_ids, n_cores, buffer_pool)
            .map_err(|e| ProofmanError::InvalidSetup(format!("{e:#}")))
    }

    /// Debugs the main and secondary state machines.
    fn debug(
        &self,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
    ) -> ProofmanResult<()> {
        for &global_id in global_ids {
            let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id)?;

            if AirClassifier::is_main(air_id) {
                MainSM::debug(&pctx, &sctx);
            } else {
                let secn_instances =
                    self.state.instance_set.secn_instances.read().map_err(|e| {
                        ProofmanError::InvalidSetup(format!("secn_instances lock poisoned: {e}"))
                    })?;
                let secn_instance = secn_instances.get(&global_id).ok_or_else(|| {
                    ProofmanError::InvalidSetup(format!(
                        "Instance not found for global_id {global_id}"
                    ))
                })?;

                secn_instance.debug(&pctx, &sctx);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_pil::MAIN_STEPS_PER_SEGMENT;

    /// `n` distinct chunks — `steps` is used only as an identity marker here.
    fn chunks(n: usize) -> Vec<Arc<EmuTrace>> {
        (0..n).map(|i| Arc::new(EmuTrace { steps: i as u64 + 1, ..EmuTrace::default() })).collect()
    }

    #[test]
    fn publish_chunks_fills_an_empty_store_up_to_idx() {
        // First release of a 4-chunk segment: the store catches up in one call.
        let traces = chunks(4);
        let mut store = Vec::new();
        publish_chunks(&mut store, &traces, 3).expect("contiguous");
        assert_eq!(store.len(), 4);
        assert!(
            store.iter().zip(&traces).all(|(a, b)| Arc::ptr_eq(a, b)),
            "chunks are published in stream order, not cloned or reordered"
        );
    }

    #[test]
    fn publish_chunks_appends_only_the_unseen_tail() {
        // Segment 1 of a num_within = 2 run: chunks 0..=1 are already published.
        let traces = chunks(4);
        let mut store: Vec<Arc<EmuTrace>> = traces[..2].to_vec();
        publish_chunks(&mut store, &traces, 3).expect("contiguous");
        assert_eq!(store.len(), 4, "only chunks 2 and 3 were appended");
        assert!(Arc::ptr_eq(&store[2], &traces[2]));
        assert!(Arc::ptr_eq(&store[3], &traces[3]));
    }

    #[test]
    fn publish_chunks_is_idempotent_per_chunk_index() {
        // Re-delivering an already-published chunk must not duplicate it.
        let traces = chunks(2);
        let mut store = Vec::new();
        publish_chunks(&mut store, &traces, 1).expect("contiguous");
        let err = publish_chunks(&mut store, &traces, 1).expect_err("already published");
        assert!(matches!(err, ExecutorError::ChunkOutOfOrder { got: 1, expected: 2 }));
        assert_eq!(store.len(), 2, "store is left untouched on rejection");
    }

    #[test]
    fn publish_chunks_rejects_a_gap() {
        // A store at 0 asked to publish chunk 1 would skip chunk 0 — reject instead
        // of leaving a hole the Main witness would read as the wrong chunk.
        let traces = chunks(3);
        let mut store: Vec<Arc<EmuTrace>> = traces[..2].to_vec();
        let err = publish_chunks(&mut store, &traces, 0).expect_err("goes backwards");
        assert!(matches!(err, ExecutorError::ChunkOutOfOrder { got: 0, expected: 2 }));
    }

    /// Replays the hook's decisions over a whole run: for each streamed chunk,
    /// publish + release when `segment_completed_by` says so. Returns the
    /// released `(segment, is_last_segment)` pairs and the final store length.
    fn replay(num_chunks: usize, num_within: usize) -> (Vec<(usize, bool)>, usize) {
        let traces = chunks(num_chunks);
        let mut store = Vec::new();
        let mut released = Vec::new();

        for idx in 0..num_chunks {
            let is_last = idx == num_chunks - 1;
            if let Some(segment) = MainPlanner::segment_completed_by(idx, num_within, is_last) {
                publish_chunks(&mut store, &traces, idx).expect("in-order stream");
                released.push((segment, is_last));
            }
        }
        (released, store.len())
    }

    #[test]
    fn replay_releases_every_segment_exactly_once_and_publishes_every_chunk() {
        let num_within = MainPlanner::traces_per_segment(CHUNK_SIZE).expect("valid chunk size");
        assert_eq!(num_within, MAIN_STEPS_PER_SEGMENT / CHUNK_SIZE as usize);

        for num_chunks in 1..=(2 * num_within + 1) {
            let (released, published) = replay(num_chunks, num_within);

            let expected: Vec<usize> = (0..num_chunks.div_ceil(num_within)).collect();
            assert_eq!(
                released.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
                expected,
                "segments released in order, once each ({num_chunks} chunks)"
            );
            assert!(
                released.iter().rev().skip(1).all(|(_, is_last)| !is_last),
                "only the final release is flagged as the last segment"
            );
            assert_eq!(
                released.last().map(|(_, is_last)| *is_last),
                Some(true),
                "the final segment is always released, partial or not"
            );
            assert_eq!(
                published, num_chunks,
                "every chunk reaches the store before its segment is released"
            );
        }
    }

    #[test]
    fn replay_publishes_a_segments_chunks_before_releasing_it() {
        // The Main witness for segment `s` reads store[s * num_within ..], so the
        // store must already cover that range at release time.
        let num_within = 4;
        let num_chunks = 10;
        let traces = chunks(num_chunks);
        let mut store = Vec::new();

        for idx in 0..num_chunks {
            let is_last = idx == num_chunks - 1;
            if let Some(segment) = MainPlanner::segment_completed_by(idx, num_within, is_last) {
                publish_chunks(&mut store, &traces, idx).expect("in-order stream");
                let start = segment * num_within;
                let end = if is_last { store.len() } else { start + num_within };
                assert!(store.len() >= end, "segment {segment} released with a short store");
                assert!(start < store.len(), "segment {segment} released empty");
            }
        }
    }
}

/// The chunks of a checkpoint, sorted: the plans list them in no particular order.
fn checkpoint_chunks(check_point: &zisk_common::CheckPoint) -> Vec<usize> {
    let mut chunks: Vec<usize> = match check_point {
        zisk_common::CheckPoint::None => Vec::new(),
        zisk_common::CheckPoint::Single(chunk) => vec![chunk.0],
        zisk_common::CheckPoint::Multiple(chunks) => chunks.iter().map(|c| c.0).collect(),
    };
    chunks.sort_unstable();
    chunks
}
