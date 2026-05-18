//! [`PlanPhase`] — pure planning actor.
//!
//! Computes main-SM plans (from minimal traces) and secondary-SM plans
//! (from per-chunk counters). **No `ProofCtx`, no MPI, no global-id
//! assignment** — that surface stays in [`crate::InstancePlanner`] for
//! now and migrates to `MaterializePhase` in M3.
//!
//! This is the executor's first fully unit-testable phase actor:
//! `plan_main` runs on synthetic `EmuTrace`s without any proof or
//! bundle bring-up.
//!
//! See `.claude/executor_refactor_plan.md` step 2.5 for context.

use std::collections::BTreeMap;

use anyhow::Result;
use fields::PrimeField64;
use sm_main::MainPlanner;
use zisk_common::{EmuTrace, Plan};

use crate::{CountersChunkMetrics, StaticSMBundle};

/// Phase-2 actor: pure planning from `ExecutionOutput` ingredients.
///
/// Holds the chunk size (used by [`MainPlanner`] to lay out main
/// instances). No state otherwise — each call is functional.
pub struct PlanPhase {
    chunk_size: u64,
}

impl PlanPhase {
    /// Construct with the chunk size — same value the trace phase used
    /// to produce the minimal traces this planner will consume.
    pub fn new(chunk_size: u64) -> Self {
        Self { chunk_size }
    }

    /// Returns the chunk size this phase was constructed with.
    #[inline]
    pub fn chunk_size(&self) -> u64 {
        self.chunk_size
    }

    /// Plan the main state-machine instances from minimal traces.
    ///
    /// Returns one [`Plan`] per `MainTrace` segment needed to cover
    /// the whole execution. No `ProofCtx`, no global ids — assignment
    /// is the caller's responsibility.
    pub fn plan_main(&self, min_traces: &[EmuTrace]) -> Result<Vec<Plan>> {
        Ok(MainPlanner::plan(min_traces, self.chunk_size)?)
    }

    /// Plan the secondary state-machine instances from per-chunk
    /// counters.
    ///
    /// The bundle's per-SM planners consume the counter map by
    /// draining via `remove`, so this method takes `&mut`. Returns a
    /// `BTreeMap` keyed by the SM's bundle position.
    pub fn plan_secondary<F: PrimeField64>(
        &self,
        counters: &mut CountersChunkMetrics,
        bundle: &StaticSMBundle<F>,
    ) -> BTreeMap<usize, Vec<Plan>> {
        bundle.plan_sec(counters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_pil::{MainTrace, MAIN_AIR_IDS, ZISK_AIRGROUP_ID};

    /// Concrete `F` placeholder for tests. We don't actually exercise
    /// any field arithmetic — just satisfy generic parameters where
    /// they appear in `MainTrace::<()>::NUM_ROWS` etc.
    const NUM_ROWS: usize = MainTrace::<()>::NUM_ROWS;

    fn synthetic_traces(n: usize) -> Vec<EmuTrace> {
        vec![EmuTrace::default(); n]
    }

    #[test]
    fn plan_main_empty_traces_yields_empty_plan() {
        let phase = PlanPhase::new(NUM_ROWS as u64);
        let plans = phase.plan_main(&[]).expect("empty traces planned ok");
        assert!(plans.is_empty());
    }

    #[test]
    fn plan_main_single_full_trace_yields_one_plan() {
        // chunk_size == NUM_ROWS → num_within = 1, so 1 trace = 1 segment.
        let phase = PlanPhase::new(NUM_ROWS as u64);
        let plans = phase.plan_main(&synthetic_traces(1)).expect("ok");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].airgroup_id, ZISK_AIRGROUP_ID);
        assert_eq!(plans[0].air_id, MAIN_AIR_IDS[0]);
    }

    #[test]
    fn plan_main_segments_via_ceil_div() {
        // chunk_size = NUM_ROWS / 2 → num_within = 2; 3 traces → 2 segments.
        let phase = PlanPhase::new((NUM_ROWS as u64) / 2);
        let plans = phase.plan_main(&synthetic_traces(3)).expect("ok");
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn plan_main_rejects_non_power_of_two_chunk_size() {
        let phase = PlanPhase::new(3);
        match phase.plan_main(&synthetic_traces(1)) {
            Ok(_) => panic!("non-power-of-two chunk_size must error"),
            Err(err) => {
                assert!(err.to_string().to_lowercase().contains("power of two"));
            }
        }
    }

    #[test]
    fn chunk_size_accessor_round_trips() {
        let phase = PlanPhase::new(1024);
        assert_eq!(phase.chunk_size(), 1024);
    }

    // Note: plan_secondary requires a real StaticSMBundle (which needs
    // Std::new(ProofCtx, SetupCtx, ...)) — integration-test territory.
    // The pure logic it dispatches into (StaticSMBundle::plan_sec /
    // sm.build_planner / per-SM planner.plan) is exercised by the
    // existing executor integration suite.
}
