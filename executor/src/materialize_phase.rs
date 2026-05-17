//! [`MaterializePhase`] — phases 2–4 of the executor pipeline.
//!
//! Consumes the [`crate::TraceOutput`] produced by [`crate::TracePhase`]
//! and runs every `ProofCtx`-mutating step that the executor needs
//! before witness computation can start:
//!
//! 1. Register the ROM instance on `pctx` (`add_instance_assign`).
//! 2. Plan main instances (via [`crate::PlanPhase::plan_main`]), assign
//!    their global ids, and populate them into `state.main_instances`.
//! 3. Stash `min_traces` in the execution state so the witness side can
//!    read them.
//! 4. Plan secondary instances (via [`crate::PlanPhase::plan_secondary`]),
//!    await the ASM Memory-Operations runner and merge its plans,
//!    await the ASM ROM-Histogram runner and hand its data to the
//!    orchestrator.
//! 5. Configure secondary SMs on `pctx`, flatten the per-SM plan map,
//!    assign secondary global ids, inject public outputs, populate
//!    secondary instances, configure their checkpoints.
//! 6. Accumulate per-type proving cost (main + per-instance + tables).
//!
//! The function is currently a single ~150-line `run` that follows the
//! original `ZiskExecutor::execute_inner` ordering verbatim — the move
//! is intentionally faithful so it can be reviewed without behaviour
//! diff. A follow-up may split it into named `step_*` helpers; for now
//! the comments mark each logical step.
//!
//! See `.claude/executor_refactor_plan.md` step 3.2 for context.

use std::sync::RwLock;
use std::time::{Duration, Instant};

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{ProofCtx, SetupCtx};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use zisk_common::{
    stats_begin, stats_end, ExecutorStatsHandle, StatsCostPerType, StatsScope, StatsType,
};
use zisk_pil::{
    MAIN_AIR_IDS, SPECIFIED_RANGES_AIR_IDS, VIRTUAL_TABLE_0_AIR_IDS, VIRTUAL_TABLE_1_AIR_IDS,
    ZISK_AIRGROUP_ID,
};
use zisk_pil::ZiskPublicValues;

use crate::{
    state::ExecutionState, InstancePlanner, InstanceRegistry, PlanPhase, TraceOutput,
    WitnessOrchestrator,
};

/// Side-information emitted by [`MaterializePhase::run`] for the caller
/// to fold into [`zisk_common::ZiskExecutorTime`] /
/// [`zisk_common::ZiskExecutorSummary`].
pub struct MaterializeOutput {
    /// Wall-clock time spent counting + planning (covers main planning
    /// through secondary planning, before the MO merge wait).
    pub count_and_plan_duration: Duration,
    /// Wall-clock time spent waiting on the MO runner and merging its
    /// plans. Near-zero on the Rust backend.
    pub count_and_plan_mo_duration: Duration,
    /// Accumulated proving cost per stats type.
    pub cost_per_type: StatsCostPerType,
}

/// Phase-3 actor — runs phases 2–4 of `ZiskExecutor::execute_inner`.
///
/// Stateless: every call to [`Self::run`] takes the dependencies it
/// needs. Constructed once at the executor level for convenience.
pub struct MaterializePhase;

impl MaterializePhase {
    /// Construct the phase. Stateless — `new()` is just a marker.
    pub fn new() -> Self {
        Self
    }

    /// Run phases 2–4. Consumes the trace output produced by
    /// [`crate::TracePhase`] and mutates the proof context, execution
    /// state, and orchestrator accordingly.
    ///
    /// The caller passes pre-built references to every co-actor
    /// (`plan`, `planner`, `registry`, `orchestrator`) plus the proof
    /// / setup contexts and the global-id collector. Returns the
    /// timing + cost side-information the caller folds into the
    /// execution summary.
    ///
    /// Note: this function currently has a long argument list; a
    /// future polish step may introduce a `MaterializeContext` to
    /// aggregate the shared borrows. Kept explicit for now to make
    /// the migration easy to read.
    #[allow(clippy::too_many_arguments)]
    // `stats` / `exec_scope` only referenced inside `stats_begin!` /
    // `stats_end!`, which expand to nothing without the `stats` feature.
    #[allow(unused_variables)]
    pub fn run<F: PrimeField64>(
        &self,
        trace: TraceOutput,
        plan: &PlanPhase,
        planner: &InstancePlanner,
        registry: &InstanceRegistry<F>,
        orchestrator: &WitnessOrchestrator<F>,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_ids: &RwLock<Vec<usize>>,
        stats: &ExecutorStatsHandle,
        exec_scope: &StatsScope,
    ) -> Result<MaterializeOutput> {
        // ────────────────────────────────────────────────────────────
        // Phase 2: plan + register + populate main instances
        // ────────────────────────────────────────────────────────────
        stats_begin!(stats, exec_scope, _main_plan_scope, "MAIN_PLAN", 0);

        timer_start_info!(PLAN);
        let start_partial = Instant::now();

        planner.assign_rom_instance(pctx)?;

        let main_plans = plan.plan_main(&trace.min_traces)?;
        *state
            .min_traces
            .write()
            .map_err(|e| anyhow::anyhow!("min_traces lock poisoned: {e}"))? =
            Some(trace.min_traces);

        let main_assignments = planner.assign_main_instances(pctx, global_ids, main_plans)?;
        let main_instances_count = main_assignments.len();
        registry.populate_main_instances(pctx, state, main_assignments)?;

        stats_end!(stats, &_main_plan_scope);

        // ────────────────────────────────────────────────────────────
        // Phase 3: plan secondary + merge MO + hand off RH
        // ────────────────────────────────────────────────────────────
        stats_begin!(stats, exec_scope, _secn_plan_scope, "SECN_PLAN", 0);

        let mut counters = trace.counters;
        let mut secn_planning = plan.plan_secondary(&mut counters, registry.sm_bundle());

        let count_and_plan_duration = start_partial.elapsed();
        timer_stop_and_log_info!(PLAN);

        timer_start_info!(WAIT_PLAN_MEM_CPP);
        stats_end!(stats, &_secn_plan_scope);
        let mo_start = Instant::now();

        // Wait for the ASM Memory Operations runner (if any) and merge
        // its plans into the secondary planning. On the Rust path
        // `await_mem_plans()` returns an empty `Vec` immediately, so
        // this is unconditional and the stats scopes fire with ~0ms
        // duration.
        let mut backend = trace.backend;
        stats_begin!(stats, exec_scope, _mo_wait_scope, "MO_PLAN_WAIT", 0);
        let mem_plans = backend.await_mem_plans()?;
        stats_end!(stats, &_mo_wait_scope);

        stats_begin!(stats, exec_scope, _mo_add_scope, "MO_PLAN_ADD", 0);
        registry.sm_bundle().extend_mem_plans(&mut secn_planning, mem_plans);
        stats_end!(stats, &_mo_add_scope);

        let count_and_plan_mo_duration = mo_start.elapsed();
        timer_stop_and_log_info!(WAIT_PLAN_MEM_CPP);

        // Wait for the ASM ROM Histogram runner (if any) and hand its
        // output to the orchestrator. `await_rom_histogram()` returns
        // `Ok(None)` on the Rust path and on non-first ASM ranks.
        timer_start_info!(WAIT_ASM_RH);
        if let Some(rh_data) = backend.await_rom_histogram()? {
            orchestrator.set_rh_data(rh_data)?;
        }
        timer_stop_and_log_info!(WAIT_ASM_RH);

        // ────────────────────────────────────────────────────────────
        // Phase 4: configure + assign secondary + publics + populate
        //          + checkpoints + cost accumulation
        // ────────────────────────────────────────────────────────────
        stats_begin!(stats, exec_scope, _config_scope, "CONFIGURE_INSTANCES", 0);

        // Configure secondary state machine instances based on planning.
        registry.configure_sm_instances(pctx, &secn_planning);

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

        planner.assign_secn_instances(pctx, global_ids, &mut secn_planning)?;

        let secn_global_ids: Vec<usize> = secn_planning
            .iter()
            .map(|plan| {
                plan.global_id
                    .ok_or_else(|| anyhow::anyhow!("secn plan missing global_id after assignment"))
            })
            .collect::<Result<Vec<_>>>()?;

        // Add public values to the proof context (Option D: pub_outs
        // flow directly from the executor output, not via the
        // planner's downcast).
        let mut publics = ZiskPublicValues::from_vec_guard(pctx.get_publics());
        for (index, value) in trace.pub_outs.0.iter() {
            publics.inputs[*index as usize] = F::from_u32(*value);
        }
        drop(publics);

        // Create secondary instances directly from the plans.
        registry.populate_secn_instances(state, secn_planning)?;

        // Configure instance checkpoints using registry method.
        registry.configure_checkpoints(pctx, state, &secn_global_ids)?;

        stats_end!(stats, &_config_scope);

        // Cost accumulation: per-secondary instance.
        let secn_instances = state
            .secn_instances
            .read()
            .map_err(|e| anyhow::anyhow!("secn_instances lock poisoned: {e}"))?;
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

        // Cost accumulation: static tables.
        let tables_air_ids =
            [SPECIFIED_RANGES_AIR_IDS[0], VIRTUAL_TABLE_0_AIR_IDS[0], VIRTUAL_TABLE_1_AIR_IDS[0]];
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

        Ok(MaterializeOutput {
            count_and_plan_duration,
            count_and_plan_mo_duration,
            cost_per_type,
        })
    }
}

impl Default for MaterializePhase {
    fn default() -> Self {
        Self::new()
    }
}
