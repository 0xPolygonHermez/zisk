//! [`PlanPhase`] — phase actor that plans, materializes, and accumulates cost.
//!
//! Consumes the [`crate::ExecutionOutput`] produced by [`crate::TracePhase`]
//! and runs every `ProofCtx`-mutating step that the executor needs
//! before witness computation can start:
//!
//! 1. Register the ROM instance on `pctx` (`add_instance_assign`).
//! 2. Plan main instances (via [`Self::plan_main`]), assign their global
//!    ids, and populate them into `state.instance_set.main_instances`.
//! 3. Stash `min_traces` in the execution state so the witness side can
//!    read them.
//! 4. Plan secondary instances (via [`Self::plan_secondary`]), await the
//!    ASM Memory-Operations runner and merge its plans, await the ASM
//!    ROM-Histogram runner and hand its data to the witness router.
//! 5. Configure secondary SMs on `pctx`, flatten the per-SM plan map,
//!    assign secondary global ids, inject public outputs, populate
//!    secondary instances, configure their checkpoints.
//! 6. Accumulate per-type proving cost (main + per-instance + tables).
//!
//! `plan_main` / `plan_secondary` are pure functions and individually
//! unit-testable on synthetic `EmuTrace` / counters input. The full
//! `run` is integration-tested via the executor pipeline.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{ProofCtx, SetupCtx};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use sm_main::MainPlanner;
use zisk_common::{
    stats_begin, stats_end, EmuTrace, ExecutorStatsHandle, Plan, StatsCostPerType, StatsScope,
    StatsType,
};
use zisk_pil::{
    MAIN_AIR_IDS, SPECIFIED_RANGES_AIR_IDS, VIRTUAL_TABLE_0_AIR_IDS, VIRTUAL_TABLE_1_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

use crate::ports::{GlobalId, ProofRegistry};
use crate::{
    state::ExecutionState, CountersChunkMetrics, ExecutionOutput, InstancePlanner,
    InstanceRegistry, StaticSMBundle, WitnessRouter,
};

/// Telemetry returned from [`PlanPhase::run`] for the executor to fold
/// into [`zisk_common::ZiskExecutorTime`] / [`zisk_common::ZiskExecutorSummary`].
///
/// The per-execution *data* (`min_traces`, `instance_set`,
/// `collector_store`) lives on [`ExecutionState`] directly — `run`
/// mutates those fields through `state` as a side-effect. This struct
/// is consumed inline by the caller and dropped.
pub struct PlanOutput {
    /// Wall-clock time spent counting + planning (covers main planning
    /// through secondary planning, before the MO merge wait).
    pub count_and_plan_duration: Duration,
    /// Wall-clock time spent waiting on the MO runner and merging its
    /// plans. Near-zero on the Rust backend.
    pub count_and_plan_mo_duration: Duration,
    /// Accumulated proving cost per stats type.
    pub cost_per_type: StatsCostPerType,
}

/// Plan the main state-machine instances from minimal traces.
///
/// Pure: returns one [`Plan`] per `MainTrace` segment needed to cover
/// the whole execution. No `ProofCtx`, no global ids — assignment is
/// the caller's responsibility. Unit-testable on synthetic `EmuTrace`
/// input without bringing up an SM bundle.
pub fn plan_main(min_traces: &[EmuTrace], chunk_size: u64) -> Result<Vec<Plan>> {
    Ok(MainPlanner::plan(min_traces, chunk_size)?)
}

/// Plan the secondary state-machine instances from per-chunk counters.
///
/// The bundle's per-SM planners consume the counter map by draining
/// via `remove`, so this function takes `&mut`. Returns a `BTreeMap`
/// keyed by the SM's bundle position.
pub fn plan_secondary<F: PrimeField64>(
    counters: &mut CountersChunkMetrics,
    bundle: &StaticSMBundle<F>,
) -> BTreeMap<usize, Vec<Plan>> {
    bundle.plan_sec(counters)
}

/// Plan + materialize phase actor.
///
/// Owns the chunk size + co-actors used by the full plan-and-populate
/// pipeline: the global-id assigner (`InstancePlanner`) and the
/// instance lifecycle owner (`InstanceRegistry`). The pure planning
/// helpers [`plan_main`] / [`plan_secondary`] are free functions in
/// this module — testable without instantiating `PlanPhase`.
pub struct PlanPhase<F: PrimeField64> {
    chunk_size: u64,
    planner: InstancePlanner,
    registry: InstanceRegistry<F>,
}

impl<F: PrimeField64> PlanPhase<F> {
    /// Construct with the chunk size and the shared SM bundle. The
    /// bundle is wrapped into an `InstanceRegistry` so the phase can
    /// populate instances during `run`.
    pub fn new(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self {
            chunk_size,
            planner: InstancePlanner::new(),
            registry: InstanceRegistry::new(sm_bundle),
        }
    }

    /// Returns the chunk size this phase was constructed with.
    #[inline]
    pub fn chunk_size(&self) -> u64 {
        self.chunk_size
    }

    /// Borrows the shared SM bundle. Exposed so the trace phase can
    /// hand it into the per-chunk counter set.
    #[inline]
    pub fn sm_bundle(&self) -> &StaticSMBundle<F> {
        self.registry.sm_bundle()
    }

    /// Run the full plan + materialize pipeline. Consumes the trace
    /// output produced by [`crate::TracePhase`] and mutates the proof
    /// context, execution state, and witness router accordingly.
    ///
    /// `proof_registry` is the executor's anti-corruption layer over
    /// `ProofCtx<F>`; it routes every `add_instance*`,
    /// `set_witness_ready`, `dctx_*`, and `write_pub_outs` call. `pctx`
    /// is still passed because `InstanceRegistry::configure_sm_instances`
    /// forwards to `StaticSMBundle::configure_instances`, which uses
    /// the SM-trait's `&ProofCtx` argument (cross-crate, kept as-is).
    #[allow(clippy::too_many_arguments)]
    // `stats` / `exec_scope` only referenced inside `stats_begin!` /
    // `stats_end!`, which expand to nothing without the `stats` feature.
    #[allow(unused_variables)]
    pub fn run(
        &self,
        trace: ExecutionOutput,
        router: &WitnessRouter<F>,
        state: &ExecutionState<F>,
        proof_registry: &dyn ProofRegistry,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_ids: &RwLock<Vec<usize>>,
        stats: &ExecutorStatsHandle,
        exec_scope: &StatsScope,
    ) -> Result<PlanOutput> {
        // ────────────────────────────────────────────────────────────
        // Phase 2: plan + register + populate main instances
        // ────────────────────────────────────────────────────────────
        stats_begin!(stats, exec_scope, _main_plan_scope, "MAIN_PLAN", 0);

        timer_start_info!(PLAN);
        let start_partial = Instant::now();

        self.planner.assign_rom_instance(proof_registry)?;

        let main_plans = plan_main(&trace.min_traces, self.chunk_size)?;
        *state
            .min_traces
            .write()
            .map_err(|e| anyhow::anyhow!("min_traces lock poisoned: {e}"))? =
            Some(trace.min_traces);

        let main_assignments =
            self.planner.assign_main_instances(proof_registry, global_ids, main_plans)?;
        let main_instances_count = main_assignments.len();
        self.registry.populate_main_instances(proof_registry, state, main_assignments)?;

        stats_end!(stats, &_main_plan_scope);

        // ────────────────────────────────────────────────────────────
        // Phase 3: plan secondary + merge MO + hand off RH
        // ────────────────────────────────────────────────────────────
        stats_begin!(stats, exec_scope, _secn_plan_scope, "SECN_PLAN", 0);

        let mut counters = trace.counters;
        let mut secn_planning = plan_secondary(&mut counters, self.registry.sm_bundle());

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
        self.registry.sm_bundle().extend_mem_plans(&mut secn_planning, mem_plans);
        stats_end!(stats, &_mo_add_scope);

        let count_and_plan_mo_duration = mo_start.elapsed();
        timer_stop_and_log_info!(WAIT_PLAN_MEM_CPP);

        // Wait for the ASM ROM Histogram runner (if any) and hand its
        // output to the witness router. `await_rom_histogram()` returns
        // `Ok(None)` on the Rust path and on non-first ASM ranks.
        timer_start_info!(WAIT_ASM_RH);
        if let Some(rh_data) = backend.await_rom_histogram()? {
            router.set_rh_data(rh_data)?;
        }
        timer_stop_and_log_info!(WAIT_ASM_RH);

        // ────────────────────────────────────────────────────────────
        // Phase 4: configure + assign secondary + publics + populate
        //          + checkpoints + cost accumulation
        // ────────────────────────────────────────────────────────────
        stats_begin!(stats, exec_scope, _config_scope, "CONFIGURE_INSTANCES", 0);

        // Configure secondary state machine instances based on planning.
        self.registry.configure_sm_instances(pctx, &secn_planning);

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

        self.planner.assign_secn_instances(proof_registry, global_ids, &mut secn_planning)?;

        let secn_global_ids: Vec<usize> = secn_planning
            .iter()
            .map(|plan| {
                plan.global_id
                    .ok_or_else(|| anyhow::anyhow!("secn plan missing global_id after assignment"))
            })
            .collect::<Result<Vec<_>>>()?;

        // Inject public outputs into the proof context. The ACL handles
        // the F-specific conversion inside `ProofmanAdapter`.
        proof_registry.write_pub_outs(&trace.pub_outs.0);

        // Create secondary instances directly from the plans.
        self.registry.populate_secn_instances(state, secn_planning)?;

        // Configure instance checkpoints.
        self.registry.configure_checkpoints(proof_registry, state, &secn_global_ids)?;

        stats_end!(stats, &_config_scope);

        // Cost accumulation: per-secondary instance.
        let secn_instances = state
            .instance_set
            .secn_instances
            .read()
            .map_err(|e| anyhow::anyhow!("secn_instances lock poisoned: {e}"))?;
        for (global_id, instance) in secn_instances.iter() {
            let info = proof_registry.instance_info(GlobalId(*global_id))?;

            let setup = sctx.get_setup(info.airgroup_id, info.air_id)?;
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

        Ok(PlanOutput { count_and_plan_duration, count_and_plan_mo_duration, cost_per_type })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_pil::{MainTrace, MAIN_AIR_IDS, ZISK_AIRGROUP_ID};

    const NUM_ROWS: usize = MainTrace::<()>::NUM_ROWS;

    fn synthetic_traces(n: usize) -> Vec<EmuTrace> {
        vec![EmuTrace::default(); n]
    }

    #[test]
    fn plan_main_empty_traces_yields_empty_plan() {
        let plans = plan_main(&[], NUM_ROWS as u64).expect("empty traces planned ok");
        assert!(plans.is_empty());
    }

    #[test]
    fn plan_main_single_full_trace_yields_one_plan() {
        // chunk_size == NUM_ROWS → num_within = 1, so 1 trace = 1 segment.
        let plans = plan_main(&synthetic_traces(1), NUM_ROWS as u64).expect("ok");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].airgroup_id, ZISK_AIRGROUP_ID);
        assert_eq!(plans[0].air_id, MAIN_AIR_IDS[0]);
    }

    #[test]
    fn plan_main_segments_via_ceil_div() {
        // chunk_size = NUM_ROWS / 2 → num_within = 2; 3 traces → 2 segments.
        let plans = plan_main(&synthetic_traces(3), (NUM_ROWS as u64) / 2).expect("ok");
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn plan_main_rejects_non_power_of_two_chunk_size() {
        match plan_main(&synthetic_traces(1), 3) {
            Ok(_) => panic!("non-power-of-two chunk_size must error"),
            Err(err) => {
                assert!(err.to_string().to_lowercase().contains("power of two"));
            }
        }
    }

    // Note: plan_secondary requires a real StaticSMBundle (which needs
    // Std::new(ProofCtx, SetupCtx, ...)) — integration-test territory.
}
