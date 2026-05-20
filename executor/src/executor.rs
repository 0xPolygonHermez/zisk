//! The `ZiskExecutor` module serves as the core orchestrator for executing the ZisK ROM program
//! and generating witness computations. It manages the execution of the state machines,
//! from initial planning to witness computation.
//!
//! This module handles both main and secondary state machines, integrating tasks such as
//! planning, configuration, and witness computation.
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
    witness::WitnessContext, AirClassifier, AsmResources, EmulatorAsm, ExecutionPhase,
    ExecutionState, PlanPhase, ProofmanAdapter, StaticSMBundle, WitnessPhase,
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
    io::ZiskStdin, stats_begin, stats_end, BusDeviceMetrics, ChunkId, ExecutorStatsHandle,
    ZiskExecutorSummary, ZiskExecutorTime,
};
use zisk_core::{ZiskRom, CHUNK_SIZE};

use anyhow::Result;

/// `(chunk_id, metrics)` pair — the per-chunk device-metrics output
/// produced by counter-phase processing.
pub(crate) type DeviceMetricsByChunk = (ChunkId, Box<dyn BusDeviceMetrics>);

/// The maximum number of steps to execute in the emulator or assembly runner.
pub(crate) const MAX_NUM_STEPS: u64 = 1 << 36;

/// The `ZiskExecutor` struct orchestrates the execution of the ZisK ROM program, managing state
/// machines, planning, and witness computation.
pub struct ZiskExecutor<F: PrimeField64> {
    /// Shared execution state.
    state: ExecutionState<F>,
    /// Phase-1 Execution. Runs the chosen emulator and produces an `ExecutionOutput`.
    execution: ExecutionPhase,
    /// Phase-2 Plan. Counts, plans, registers and populates instances.
    plan: PlanPhase<F>,
    /// Phase-3 Witness computation.
    witness: WitnessPhase<F>,
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
    /// * `is_asm_emulator` - Whether to use the ASM emulator for execution
    pub fn new(
        wcm: &WitnessManager<F>,
        verbose_mode: proofman_common::VerboseMode,
        shared_tables: bool,
        is_asm_emulator: bool,
    ) -> Result<Arc<Self>> {
        let rank_info = wcm.get_rank_info();
        proofman_common::initialize_logger(verbose_mode, Some(&rank_info));

        let std = pil_std_lib::Std::new(wcm.get_pctx(), wcm.get_sctx(), shared_tables)?;
        proofman::register_std(wcm, &std);

        let precompiles = crate::Precompiles::all(std.clone());
        let sm_bundle = Arc::new(StaticSMBundle::new(std, is_asm_emulator, precompiles));

        let executor = Arc::new(Self {
            state: ExecutionState::new(),
            execution: ExecutionPhase::new(CHUNK_SIZE, is_asm_emulator),
            plan: PlanPhase::new(CHUNK_SIZE, sm_bundle.clone()),
            witness: if is_asm_emulator {
                WitnessPhase::new_asm(CHUNK_SIZE, sm_bundle)
            } else {
                WitnessPhase::new_native(CHUNK_SIZE, sm_bundle)
            },
        });

        wcm.register_component(executor.clone());
        wcm.set_witness_initialized();

        Ok(executor)
    }

    /// Sets the ZisK ROM (ELF) for execution.
    ///
    /// This method allows changing the ROM between executions without
    /// recreating the executor, making the executor more reusable.
    ///
    /// # Arguments
    /// * `zisk_rom` - The ZisK ROM to execute.
    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>, use_hints: bool) -> Result<()> {
        self.state.set_rom(zisk_rom.clone(), use_hints);
        self.witness.set_rom(zisk_rom)
    }

    /// Sets whether to use packed representation for witness computation.
    pub fn set_packed(&self, packed: bool) {
        self.witness.set_packed(packed);
    }

    /// Sets the standard input for execution.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.execution.set_stdin(stdin)
    }

    /// Sets ASM resources for execution (only applicable for ASM emulator).
    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> Result<()> {
        self.execution.set_asm_resources(asm_resources)
    }

    /// Clears any previously-installed ASM resources. No-op when the
    /// executor was built with the Rust emulator backend.
    pub fn clear_asm_resources(&self) -> Result<()> {
        // self.trace.clear_asm_resources()
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
    /// Uses `anyhow::Result` so the body can use `?` freely; the trait-method
    /// wrapper maps any error to `ProofmanError::InvalidSetup` once at the FFI seam.
    fn execute_inner(
        &self,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &RwLock<Vec<usize>>,
    ) -> Result<()> {
        let start_total = Instant::now();
        self.state.reset();
        self.witness.reset()?;

        stats_begin!(self.state.stats, 0, _exec_scope, "EXECUTE", 0);

        // Set the start time of the current execution
        self.state.stats.set_start_time(Instant::now());

        // Phase 1: Execute ROM to collect minimal traces
        timer_start_info!(COMPUTE_MINIMAL_TRACE);
        let start_partial = Instant::now();

        let zisk_rom = self.state.get_rom()?;
        let output = self.execution.run(
            &zisk_rom,
            &pctx,
            self.plan.sm_bundle(),
            self.state.use_hints.load(std::sync::atomic::Ordering::SeqCst),
            &self.state.stats,
            &_exec_scope,
        )?;

        let execution_duration = start_partial.elapsed();
        timer_stop_and_log_info!(COMPUTE_MINIMAL_TRACE);

        // Phases 2-4: planning + pctx mutation + instance materialization
        // + cost accumulation. Lifted into `MaterializePhase` in step 3.2;
        // pctx mutations route through `ProofmanAdapter` since step 3.3.
        let steps = output.steps;
        let proof_registry = ProofmanAdapter::new(&pctx);
        let artifacts = self.plan.run(
            output,
            &self.witness,
            &self.state,
            &proof_registry,
            &pctx,
            &sctx,
            global_ids,
            &self.state.stats,
            &_exec_scope,
        )?;

        // Reset hints stream and input shmem after the ASM
        // backend-specific await calls have drained the runners.
        self.execution.reset()?;

        stats_end!(self.state.stats, &_exec_scope);

        let zisk_execution_time = ZiskExecutorTime {
            execution_duration: execution_duration.as_millis() as u64,
            count_and_plan_duration: artifacts.count_and_plan_duration.as_millis() as u64,
            count_and_plan_mo_duration: artifacts.count_and_plan_mo_duration.as_millis() as u64,
            total_duration: start_total.elapsed().as_millis() as u64,
            asm_execution_duration: self.execution.get_asm_execution_info()?,
        };
        let execution_result =
            ZiskExecutorSummary::new(steps, zisk_execution_time, artifacts.cost_per_type);

        // Store the execution result
        self.state.set_execution_result(execution_result);

        Ok(())
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
    ) -> Result<()> {
        if stage != 1 {
            return Ok(());
        }

        stats_begin!(self.state.stats, 0, _witness_scope, "CALCULATE_WITNESS", 0);

        let pool = create_pool(n_cores);
        let registry = ProofmanAdapter::new(&pctx);
        pool.install(|| -> Result<()> {
            let ctx = WitnessContext::new(
                &pctx,
                &sctx,
                &self.state,
                buffer_pool,
                &_witness_scope,
                &registry,
            );
            for &global_id in global_ids {
                self.witness.dispatch(&ctx, global_id)?;
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
        _sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        _buffer_pool: &dyn BufferPool<F>,
    ) -> Result<()> {
        stats_begin!(self.state.stats, 0, _pre_scope, "PRE_CALCULATE_WITNESS", 0);

        if stage != 1 {
            return Ok(());
        }

        let pool = create_pool(n_cores);
        let registry = ProofmanAdapter::new(&pctx);

        pool.install(|| self.witness.pre_calculate(&pctx, &registry, &self.state, global_ids))?;

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
        self.execute_inner(pctx, sctx, global_ids)
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
