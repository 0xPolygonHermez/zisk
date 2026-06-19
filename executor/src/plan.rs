//! [`PlanPhase`] — pure planning over `ExecutionOutput`.
//!
//! Split into [`PlanPhase::run_main`] and [`PlanPhase::run_secondary`].
//! Instance materialization + SM configuration live on [`crate::WitnessPhase`];
//! cost accounting lives on [`crate::ProofmanAdapter`].

mod assigner;

pub use assigner::*;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::error::ExecutorResult;
use fields::PrimeField64;
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use sm_main::MainPlanner;
use zisk_common::{stats_begin, stats_end, EmuTrace, ExecutorStatsHandle, Plan, StatsScope};

use crate::sm::{extend_mem_plans, plan_sec};
use crate::{BackendArtifacts, CountersChunkMetrics};

/// Telemetry returned by [`PlanPhase::run_secondary`].
pub struct SecondaryPlanArtifacts {
    /// Secondary plans, keyed by bundle position, pre-flatten, pre-GID.
    pub secn_planning: BTreeMap<usize, Vec<Plan>>,
    /// Wall-clock for count + plan_secondary, before the MO merge wait.
    pub count_and_plan_duration: Duration,
    /// Wall-clock waiting on the MO runner and merging its plans.
    pub count_and_plan_mo_duration: Duration,
}

/// Pure-planning phase actor. Owns chunk size only.
pub struct PlanPhase<F: PrimeField64> {
    chunk_size: u64,
    _marker: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> PlanPhase<F> {
    pub fn new(chunk_size: u64) -> Self {
        Self { chunk_size, _marker: std::marker::PhantomData }
    }

    /// Plan the main SM instances. Pure: unit-testable on synthetic `EmuTrace`.
    pub fn plan_main(min_traces: &[EmuTrace], chunk_size: u64) -> ExecutorResult<Vec<Plan>> {
        Ok(MainPlanner::plan(min_traces, chunk_size)?)
    }

    /// Plan the secondary SM instances from per-chunk counters via static dispatch.
    pub fn plan_secondary(
        counters: &mut CountersChunkMetrics,
        num_chunks: usize,
        is_asm_emulator: bool,
    ) -> BTreeMap<usize, Vec<Plan>> {
        plan_sec::<F>(counters, num_chunks, is_asm_emulator)
    }

    /// Plan main only. No bundle / pctx / registry contact.
    #[allow(unused_variables)]
    pub fn run_main(
        &self,
        min_traces: &[EmuTrace],
        stats: &ExecutorStatsHandle,
        exec_scope: &StatsScope,
    ) -> ExecutorResult<Vec<Plan>> {
        stats_begin!(stats, exec_scope, _main_plan_scope, "MAIN_PLAN", 0);
        timer_start_info!(PLAN_MAIN);
        let plans = Self::plan_main(min_traces, self.chunk_size)?;
        timer_stop_and_log_info!(PLAN_MAIN);
        stats_end!(stats, &_main_plan_scope);
        Ok(plans)
    }

    /// Plan secondary + await `await_mem_plans` + merge. No bundle / pctx / registry contact.
    #[allow(unused_variables)]
    pub fn run_secondary(
        &self,
        counters: &mut CountersChunkMetrics,
        num_chunks: usize,
        is_asm_emulator: bool,
        backend: &mut BackendArtifacts,
        stats: &ExecutorStatsHandle,
        exec_scope: &StatsScope,
    ) -> ExecutorResult<SecondaryPlanArtifacts> {
        stats_begin!(stats, exec_scope, _secn_plan_scope, "SECN_PLAN", 0);
        timer_start_info!(PLAN_SECONDARY);
        let start_partial = Instant::now();

        let mut secn_planning = Self::plan_secondary(counters, num_chunks, is_asm_emulator);

        let count_and_plan_duration = start_partial.elapsed();
        timer_stop_and_log_info!(PLAN_SECONDARY);
        stats_end!(stats, &_secn_plan_scope);

        timer_start_info!(WAIT_PLAN_MEM_CPP);
        let mo_start = Instant::now();

        stats_begin!(stats, exec_scope, _mo_wait_scope, "MO_PLAN_WAIT", 0);
        let mem_plans = backend.await_mem_plans()?;
        stats_end!(stats, &_mo_wait_scope);

        stats_begin!(stats, exec_scope, _mo_add_scope, "MO_PLAN_ADD", 0);
        extend_mem_plans(&mut secn_planning, mem_plans);
        stats_end!(stats, &_mo_add_scope);

        let count_and_plan_mo_duration = mo_start.elapsed();
        timer_stop_and_log_info!(WAIT_PLAN_MEM_CPP);

        Ok(SecondaryPlanArtifacts {
            secn_planning,
            count_and_plan_duration,
            count_and_plan_mo_duration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fields::Goldilocks;
    use zisk_pil::{MainTrace, MAIN_AIR_IDS, ZISK_AIRGROUP_ID};

    type F = Goldilocks;

    const NUM_ROWS: usize = MainTrace::<()>::NUM_ROWS;

    fn synthetic_traces(n: usize) -> Vec<EmuTrace> {
        vec![EmuTrace::default(); n]
    }

    #[test]
    fn plan_main_empty_traces_yields_empty_plan() {
        let plans =
            PlanPhase::<F>::plan_main(&[], NUM_ROWS as u64).expect("empty traces planned ok");
        assert!(plans.is_empty());
    }

    #[test]
    fn plan_main_single_full_trace_yields_one_plan() {
        let plans = PlanPhase::<F>::plan_main(&synthetic_traces(1), NUM_ROWS as u64).expect("ok");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].airgroup_id, ZISK_AIRGROUP_ID);
        assert_eq!(plans[0].air_id, MAIN_AIR_IDS[0]);
    }

    #[test]
    fn plan_main_segments_via_ceil_div() {
        let plans =
            PlanPhase::<F>::plan_main(&synthetic_traces(3), (NUM_ROWS as u64) / 2).expect("ok");
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn plan_main_rejects_non_power_of_two_chunk_size() {
        match PlanPhase::<F>::plan_main(&synthetic_traces(1), 3) {
            Ok(_) => panic!("non-power-of-two chunk_size must error"),
            Err(err) => {
                assert!(err.to_string().to_lowercase().contains("power of two"));
            }
        }
    }
}
