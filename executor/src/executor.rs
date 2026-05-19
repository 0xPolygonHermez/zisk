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
    state::ExecutionState, witness_orchestrator::WitnessContext, AirClassifier, AsmResources,
    EmulatorAsm, InstancePlanner, InstanceRegistry, RomExecutor, StaticSMBundle,
    WitnessOrchestrator,
};
use fields::PrimeField64;
use proofman_common::{create_pool, BufferPool, ProofCtx, ProofmanResult, SetupCtx};
use proofman_starks_lib_c::unified_buffer_release_c;
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use sm_main::MainSM;
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};
use witness::WitnessComponent;
use zisk_common::{
    io::ZiskStdin, stats_begin, stats_end, BusDeviceMetrics, ChunkId, ExecutorStatsHandle,
    StatsCostPerType, StatsType, ZiskExecutorSummary, ZiskExecutorTime,
};
use zisk_core::{ZiskRom, CHUNK_SIZE};
use zisk_pil::ZiskPublicValues;
use zisk_pil::{
    MAIN_AIR_IDS, VIRTUAL_TABLE_ZISK_0_AIR_IDS, VIRTUAL_TABLE_ZISK_1_AIR_IDS, ZISK_AIRGROUP_ID,
};

use anyhow::Result;

pub type DeviceMetricsByChunk = (ChunkId, Box<dyn BusDeviceMetrics>); // (chunk_id, metrics)

/// The maximum number of steps to execute in the emulator or assembly runner.
pub const MAX_NUM_STEPS: u64 = 1 << 36;

/// The `ZiskExecutor` struct orchestrates the execution of the ZisK ROM program, managing state
/// machines, planning, and witness computation.
pub struct ZiskExecutor<F: PrimeField64> {
    /// Shared execution state.
    state: ExecutionState<F>,
    /// ROM executor component.
    rom_executor: RomExecutor,
    /// Instance planner component.
    planner: InstancePlanner,
    /// Instance registry component.
    registry: InstanceRegistry<F>,
    /// Witness orchestrator component.
    orchestrator: WitnessOrchestrator<F>,
}

impl<F: PrimeField64> ZiskExecutor<F> {
    /// Creates a new instance of the `ZiskExecutor`.
    ///
    /// The ROM can be set or changed via `set_rom()` before calling `execute()`.
    ///
    /// # Arguments
    /// * `std` - Standard library instance.
    /// * `sm_bundle` - State machine bundle.
    /// * `chunk_size` - Chunk size for processing.
    /// * `hints_stream` - Optional hints stream for processing precompile hints.
    #[allow(clippy::too_many_arguments)]
    pub fn new(sm_bundle: StaticSMBundle<F>) -> Self {
        let sm_bundle = Arc::new(sm_bundle);
        let chunk_size = CHUNK_SIZE;

        Self {
            state: ExecutionState::new(),
            rom_executor: RomExecutor::new(chunk_size),
            planner: InstancePlanner::new(chunk_size),
            registry: InstanceRegistry::new(sm_bundle.clone()),
            orchestrator: WitnessOrchestrator::new(chunk_size, sm_bundle),
        }
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
        self.orchestrator.set_rom(zisk_rom)
    }

    /// Sets whether to use packed representation for witness computation.
    pub fn set_packed(&self, packed: bool) {
        self.orchestrator.set_packed(packed);
    }

    /// Sets the standard input for execution.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.rom_executor.set_stdin(stdin)
    }

    /// Sets ASM resources for execution (only applicable for ASM emulator).
    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> Result<()> {
        self.rom_executor.set_asm_resources(asm_resources)
    }

    /// Clears the ASM-execution flag so execution routes through the Rust emulator.
    pub fn clear_asm_resources(&self) {
        self.rom_executor.clear_asm_resources();
    }

    /// Returns a reference to the ASM emulator if ASM execution is active.
    pub fn asm_emulator(&self) -> Option<&EmulatorAsm> {
        self.rom_executor.asm_emulator()
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
}

impl<F: PrimeField64> WitnessComponent<F> for ZiskExecutor<F> {
    /// Executes the ZisK ROM program and calculate the plans for main and secondary state machines.
    fn execute(
        &self,
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
        global_ids: &RwLock<Vec<usize>>,
    ) -> ProofmanResult<()> {
        let start_total = Instant::now();
        self.state.reset();
        self.orchestrator
            .reset()
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;

        stats_begin!(self.state.stats, 0, _exec_scope, "EXECUTE", 0);

        // Set the start time of the current execution
        self.state.stats.set_start_time(Instant::now());

        // Phase 1: Execute ROM to collect minimal traces
        timer_start_info!(COMPUTE_MINIMAL_TRACE);
        let start_partial = Instant::now();

        let zisk_rom = self
            .state
            .get_rom()
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(e.to_string()))?;
        let output = self
            .rom_executor
            .execute(
                &zisk_rom,
                &pctx,
                self.registry.sm_bundle(),
                self.state.use_hints.load(std::sync::atomic::Ordering::SeqCst),
                &self.state.stats,
                &_exec_scope,
            )
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(e.to_string()))?;

        let execution_duration = start_partial.elapsed();
        timer_stop_and_log_info!(COMPUTE_MINIMAL_TRACE);

        // Phase 2: Plan main instances
        stats_begin!(self.state.stats, &_exec_scope, _main_plan_scope, "MAIN_PLAN", 0);

        timer_start_info!(PLAN);
        let start_partial = Instant::now();

        self.planner
            .assign_rom_instance(&pctx)
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;

        let main_output = self.planner.plan_main::<F>(&output.min_traces, output.main_count);
        *self.state.min_traces.write().map_err(|e| {
            proofman_common::ProofmanError::InvalidSetup(format!("min_traces lock poisoned: {e}"))
        })? = Some(output.min_traces);

        let main_assignments = self
            .planner
            .assign_main_instances(&pctx, global_ids, main_output.plans)
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;
        let main_instances_count = main_assignments.len();
        self.registry
            .populate_main_instances(&pctx, &self.state, main_assignments)
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;

        stats_end!(self.state.stats, &_main_plan_scope);

        // Phase 3: Plan secondary instances
        stats_begin!(self.state.stats, &_exec_scope, _secn_plan_scope, "SECN_PLAN", 0);

        let mut secn_count = output.secn_count;
        let mut secn_planning = self.planner.plan_secondary(
            self.registry.sm_bundle(),
            &mut secn_count,
            self.rom_executor.is_asm_emulator(),
        );

        let count_and_plan_duration = start_partial.elapsed();
        timer_stop_and_log_info!(PLAN);

        timer_start_info!(WAIT_PLAN_MEM_CPP);
        stats_end!(self.state.stats, &_secn_plan_scope);
        let start_partial = Instant::now();

        // Handle memory operations from ASM runner
        if let Some(handle_mo) = output.handle_mo {
            stats_begin!(self.state.stats, &_exec_scope, _mo_wait_scope, "MO_PLAN_WAIT", 0);

            let asm_runner_mo = handle_mo
                .join()
                .map_err(|_| {
                    proofman_common::ProofmanError::InvalidSetup(
                        "Assembly Memory Operations thread panicked".to_string(),
                    )
                })
                .and_then(|r| {
                    r.map_err(|e| {
                        proofman_common::ProofmanError::InvalidSetup(format!(
                            "Assembly Memory Operations execution failed: {e}"
                        ))
                    })
                })?;

            stats_end!(self.state.stats, &_mo_wait_scope);
            stats_begin!(self.state.stats, &_exec_scope, _mo_add_scope, "MO_PLAN_ADD", 0);

            secn_planning
                .entry(self.registry.sm_bundle().get_mem_sm_id())
                .or_default()
                .extend(asm_runner_mo.plans);

            stats_end!(self.state.stats, &_mo_add_scope);

            // Release proofman's borrowed unified buffer: the MO
            // count-and-plan GPU work is now done (handle_mo joined
            // just above).
            if pctx.gpu {
                unified_buffer_release_c(pctx.get_device_buffers_ptr());
            }
        }

        let count_and_plan_mo_duration = start_partial.elapsed();
        timer_stop_and_log_info!(WAIT_PLAN_MEM_CPP);

        if let Some(handle_rh) = output.handle_rh {
            timer_start_info!(WAIT_ASM_RH);
            let rh_data = handle_rh
                .join()
                .map_err(|_| {
                    proofman_common::ProofmanError::InvalidSetup(
                        "ROM Histogram thread panicked".to_string(),
                    )
                })
                .and_then(|r| {
                    r.map_err(|e| {
                        proofman_common::ProofmanError::InvalidSetup(format!(
                            "ROM Histogram execution failed: {e}"
                        ))
                    })
                })?;

            self.orchestrator
                .set_rh_data(rh_data)
                .map_err(|e| proofman_common::ProofmanError::InvalidSetup(e.to_string()))?;

            timer_stop_and_log_info!(WAIT_ASM_RH);
        }

        // Phase 4: Configure and assign secondary instances
        stats_begin!(self.state.stats, &_exec_scope, _config_scope, "CONFIGURE_INSTANCES", 0);

        // Configure secondary state machine instances based on planning
        self.registry.configure_sm_instances(&pctx, &secn_planning);

        let mut cost_per_type = StatsCostPerType::default();
        {
            let setup_main = sctx.get_setup(ZISK_AIRGROUP_ID, MAIN_AIR_IDS[0])?;
            let n_bits = setup_main.stark_info.stark_struct.n_bits;
            let total_cols: u64 = setup_main
                .stark_info
                .map_sections_n
                .iter()
                .filter(|(key, _)| *key != "const")
                .map(|(_, value)| *value)
                .sum();
            let cost = (1 << n_bits) * total_cols;
            cost_per_type.add_cost(StatsType::Main, cost * main_instances_count as u64);
        }

        let mut secn_planning: Vec<_> = secn_planning.into_values().flatten().collect();

        self.planner
            .assign_secn_instances(&pctx, global_ids, &mut secn_planning)
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;

        let secn_global_ids: Vec<usize> = secn_planning
            .iter()
            .map(|plan| {
                plan.global_id.ok_or_else(|| {
                    proofman_common::ProofmanError::InvalidSetup(
                        "secn plan missing global_id after assignment".to_string(),
                    )
                })
            })
            .collect::<ProofmanResult<Vec<_>>>()?;

        // Add public values to the proof context
        let mut publics = ZiskPublicValues::from_vec_guard(pctx.get_publics());
        for (index, value) in main_output.public_values.iter() {
            publics.inputs[*index as usize] = F::from_u32(*value);
        }
        drop(publics);

        // Store secondary planning in execution state
        *self.state.secn_planning.write().map_err(|e| {
            proofman_common::ProofmanError::InvalidSetup(format!(
                "secn_planning lock poisoned: {e}"
            ))
        })? = secn_planning;

        // Create secondary instances
        self.registry
            .populate_secn_instances(&self.state, &secn_global_ids)
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;

        // Configure instance checkpoints using registry method
        self.registry
            .configure_checkpoints(&pctx, &self.state, &secn_global_ids)
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;

        // Reset hints stream and input shmem
        self.rom_executor
            .reset()
            .map_err(|e| proofman_common::ProofmanError::InvalidSetup(e.to_string()))?;

        stats_end!(self.state.stats, &_config_scope);
        stats_end!(self.state.stats, &_exec_scope);

        let secn_instances = self.state.secn_instances.read().map_err(|e| {
            proofman_common::ProofmanError::InvalidSetup(format!(
                "secn_instances lock poisoned: {e}"
            ))
        })?;
        for (global_id, instance) in secn_instances.iter() {
            let (airgroup_id, air_id) = pctx.dctx_get_instance_info(*global_id)?;

            let setup = sctx.get_setup(airgroup_id, air_id)?;
            let n_bits = setup.stark_info.stark_struct.n_bits;
            let total_cols: u64 = setup
                .stark_info
                .map_sections_n
                .iter()
                .filter(|(key, _)| *key != "const")
                .map(|(_, value)| *value)
                .sum();
            let cost = (1 << n_bits) * total_cols;
            let stats_type = instance.stats_type();
            cost_per_type.add_cost(stats_type, cost);
        }

        let tables_air_ids = [VIRTUAL_TABLE_ZISK_0_AIR_IDS[0], VIRTUAL_TABLE_ZISK_1_AIR_IDS[0]];
        for air_id in tables_air_ids {
            let setup = sctx.get_setup(ZISK_AIRGROUP_ID, air_id)?;
            let n_bits = setup.stark_info.stark_struct.n_bits;
            let total_cols: u64 = setup
                .stark_info
                .map_sections_n
                .iter()
                .filter(|(key, _)| *key != "const")
                .map(|(_, value)| *value)
                .sum();
            let cost = (1 << n_bits) * total_cols;
            cost_per_type.add_cost(StatsType::Tables, cost);
        }

        let zisk_execution_time = ZiskExecutorTime {
            execution_duration: execution_duration.as_millis() as u64,
            count_and_plan_duration: count_and_plan_duration.as_millis() as u64,
            count_and_plan_mo_duration: count_and_plan_mo_duration.as_millis() as u64,
            total_duration: start_total.elapsed().as_millis() as u64,
            asm_execution_duration: self
                .rom_executor
                .get_asm_execution_info()
                .map_err(|e| proofman_common::ProofmanError::InvalidSetup(e.to_string()))?,
        };
        // Store the execution result
        let execution_result =
            ZiskExecutorSummary::new(output.steps, zisk_execution_time, cost_per_type);

        // Store the execution result
        self.state.set_execution_result(execution_result);

        Ok(())
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
        if stage != 1 {
            return Ok(());
        }

        stats_begin!(self.state.stats, 0, _witness_scope, "CALCULATE_WITNESS", 0);

        let pool = create_pool(n_cores);
        pool.install(|| -> ProofmanResult<()> {
            let ctx = WitnessContext::new(
                &pctx,
                &sctx,
                &self.state,
                buffer_pool,
                &_witness_scope,
                self.rom_executor.is_asm_emulator(),
            );
            for &global_id in global_ids {
                self.orchestrator
                    .compute_witness_for_instance(&ctx, global_id)
                    .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;
            }
            Ok(())
        })?;

        stats_end!(self.state.stats, &_witness_scope);

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
        stats_begin!(self.state.stats, 0, _pre_scope, "PRE_CALCULATE_WITNESS", 0);

        if stage != 1 {
            return Ok(());
        }

        let pool = create_pool(n_cores);

        pool.install(|| {
            self.orchestrator.pre_calculate(
                &pctx,
                &self.state,
                global_ids,
                self.rom_executor.is_asm_emulator(),
            )
        })
        .map_err(|e| proofman_common::ProofmanError::InvalidSetup(format!("{e:#}")))?;

        stats_end!(self.state.stats, &_pre_scope);
        Ok(())
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
                let secn_instances = self.state.secn_instances.read().map_err(|e| {
                    proofman_common::ProofmanError::InvalidSetup(format!(
                        "secn_instances lock poisoned: {e}"
                    ))
                })?;
                let secn_instance = secn_instances.get(&global_id).ok_or_else(|| {
                    proofman_common::ProofmanError::InvalidSetup(format!(
                        "Instance not found for global_id {global_id}"
                    ))
                })?;

                secn_instance.debug(&pctx, &sctx);
            }
        }
        Ok(())
    }
}
