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
    ports::ProofRegistry, witness::WitnessContext, AirClassifier, AsmResources, EmulatorAsm,
    ExecutionPhase, ExecutionState, InstanceAssigner, NoopProofRegistry, PlanPhase,
    ProofmanAdapter, StaticSMBundle, WitnessPhase,
};
use fields::PrimeField64;
use proofman_common::{create_pool, BufferPool, ProofCtx, ProofmanError, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use sm_main::MainSM;
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};
use witness::{WitnessComponent, WitnessManager};
use zisk_common::{
    io::ZiskStdin, stats_begin, stats_end, AirInstanceCount, BusDeviceMetrics, ChunkId,
    ExecutorStatsHandle, Plan, ZiskExecutorSummary, ZiskExecutorTime,
};
use zisk_core::{ZiskRom, CHUNK_SIZE};

use crate::error::{ExecutorResult, RwLockExt};

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

        let std = pil_std_lib::Std::new(wcm.get_pctx(), wcm.get_sctx(), shared_tables)?;
        proofman::register_std(wcm, &std);

        let precompiles = crate::Precompiles::all(std.clone());
        let sm_bundle = Arc::new(StaticSMBundle::new(std, precompiles));

        let executor = Arc::new(Self {
            state: ExecutionState::new(),
            execution: ExecutionPhase::new(CHUNK_SIZE, with_asm_emulator),
            plan: PlanPhase::new(CHUNK_SIZE),
            witness: Some(WitnessPhase::new(CHUNK_SIZE, sm_bundle)),
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

    /// Sets whether to use packed representation for witness computation.
    pub fn set_packed(&self, packed: bool) {
        if let Some(witness) = self.witness.as_ref() {
            witness.set_packed(packed);
        }
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
        self.state.reset();
        if let Some(witness) = self.witness.as_ref() {
            witness.reset()?;
        }

        stats_begin!(self.state.stats, 0, _exec_scope, "EXECUTE", 0);
        self.state.stats.set_start_time(Instant::now());

        let is_asm_emulator = self.execution.is_asm_execution();

        // Reserve proofman's unified GPU buffer for MO count-and-plan
        // (no-op on CPU / standalone).
        if is_asm_emulator {
            if let Some(extras) = proofman_extras {
                extras.acquire_gpu_buffer();
            }
        }

        // ────────────────────────────────────────────────────────────
        // Phase 1.1: Emulate
        // ────────────────────────────────────────────────────────────
        timer_start_info!(COMPUTE_MINIMAL_TRACE);
        let start_partial = Instant::now();

        let zisk_rom = self.state.get_rom()?;
        let stdin = self.state.get_stdin();
        let output = self.execution.run::<F>(
            &zisk_rom,
            &stdin,
            registry.is_first_process(),
            self.state.use_hints.load(std::sync::atomic::Ordering::SeqCst),
            &self.state.stats,
            &_exec_scope,
        )?;

        let execution_duration = start_partial.elapsed();
        timer_stop_and_log_info!(COMPUTE_MINIMAL_TRACE);

        // ────────────────────────────────────────────────────────────
        // Phase 1.2: Plan + assign main, then populate main (witness only)
        // ────────────────────────────────────────────────────────────
        let steps = output.steps;

        let crate::ExecutionOutput { min_traces, mut counters, pub_outs, mut backend, .. } = output;
        let num_chunks = min_traces.len();

        InstanceAssigner::assign_rom_instance(registry)?;
        let main_plans = self.plan.run_main(&min_traces, &self.state.stats, &_exec_scope)?;
        *self.state.min_traces.write_or_poison("min_traces")? = Some(min_traces);

        let main_assignments =
            InstanceAssigner::assign_main_instances(registry, global_ids, main_plans)?;
        let main_instances_count = main_assignments.len();
        if let Some(witness) = self.witness.as_ref() {
            witness.populate_main_instances(registry, &self.state, main_assignments)?;
        }

        // ────────────────────────────────────────────────────────────
        // Phase 1.3: Plan secondary, await async, configure + populate (witness only)
        // ────────────────────────────────────────────────────────────
        let secn_artifacts = self.plan.run_secondary(
            &mut counters,
            num_chunks,
            is_asm_emulator,
            &mut backend,
            &self.state.stats,
            &_exec_scope,
        )?;

        // MO runner joined in `run_secondary`; release the buffer back to proofman.
        // Earlier error paths skip the release on purpose: the MO thread may
        // still be using the buffer (see `release_gpu_buffer` docs).
        if is_asm_emulator {
            if let Some(extras) = proofman_extras {
                extras.release_gpu_buffer();
            }
        }

        timer_start_info!(WAIT_ASM_RH);
        if let Some(rh_data) = backend.await_rom_histogram()? {
            if let Some(witness) = self.witness.as_ref() {
                witness.set_rh_data(rh_data)?;
            }
        }
        timer_stop_and_log_info!(WAIT_ASM_RH);

        stats_begin!(self.state.stats, &_exec_scope, _config_scope, "CONFIGURE_INSTANCES", 0);

        if let (Some(witness), Some(extras)) = (self.witness.as_ref(), proofman_extras) {
            witness.configure_sm_instances(extras.pctx(), &secn_artifacts.secn_planning);
        }

        let mut secn_plans: Vec<Plan> =
            secn_artifacts.secn_planning.into_values().flatten().collect();
        InstanceAssigner::assign_secn_instances(registry, global_ids, &mut secn_plans)?;
        let secn_global_ids: Vec<usize> = secn_plans
            .iter()
            .map(|plan| {
                plan.global_id
                    .ok_or(crate::error::ExecutorError::SecnPlanMissing { phase: "assignment" })
            })
            .collect::<ExecutorResult<Vec<_>>>()?;

        registry.write_pub_outs(&pub_outs.0);

        if let Some(witness) = self.witness.as_ref() {
            witness.populate_secn_instances(&self.state, secn_plans)?;
            witness.configure_checkpoints(registry, &self.state, &secn_global_ids)?;
        }

        stats_end!(self.state.stats, &_config_scope);

        // Reset hints stream and input shmem after the ASM
        // backend-specific await calls have drained the runners.
        self.execution.reset()?;

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
            count_and_plan_duration: secn_artifacts.count_and_plan_duration.as_millis() as u64,
            count_and_plan_mo_duration: secn_artifacts.count_and_plan_mo_duration.as_millis()
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

        let pool = create_pool(n_cores);
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

        let pool = create_pool(n_cores);
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
