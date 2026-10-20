//! Centralised emission of coordinator metrics.
//!
//! All metric *descriptions* live in `zisk-coordinator-server`'s `metrics` module (alongside the
//! Prometheus recorder install). This module emits values through the `metrics` facade and keeps
//! the [`journal`] of recent jobs and events; [`Coordinator::render_metrics`] adds what is read
//! from the coordinator's state when `/metrics` is scraped.
//!
//! [`Coordinator::render_metrics`]: crate::Coordinator::render_metrics

mod journal;
pub(crate) mod worker_report;

use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use zisk_cluster_common::{JobId, WorkerId, WorkerState};
use zisk_common::{StatsCostPerType, ZiskExecutorTime};

pub(crate) use journal::{
    JobEnded, Journal, PHASE_AGGREGATION, PHASE_CONTRIBUTIONS, PHASE_EXECUTION, PHASE_PROVE,
};

/// Closed set of `outcome` label values. Sharing the constants between the
/// helper and its callers keeps the label cardinality fixed and avoids drift
/// against the Prometheus descriptions in `zisk-coordinator-server`.
pub(crate) const OUTCOME_SUCCESS: &str = "success";
pub(crate) const OUTCOME_FAILURE: &str = "failure";
pub(crate) const OUTCOME_CANCELLED: &str = "cancelled";

/// Closed set of `kind` label values.
pub(crate) const KIND_PROVE: &str = "prove";
pub(crate) const KIND_EXECUTE: &str = "execute";
pub(crate) const KIND_WRAP: &str = "wrap";

/// What a failed job's reason falls under ([`failure_class`]).
const FAILURE_CLASSES: [&str; 3] = ["timeout", "worker", "other"];
/// What a job's cost is split into.
const COST_TYPES: [&str; 6] = ["main", "opcode", "memory", "precompile", "tables", "other"];
/// A job that ended without having been recorded as started.
const KIND_UNKNOWN: &str = "unknown";

static JOURNAL: LazyLock<Journal> = LazyLock::new(Journal::default);

/// The coordinator's recent jobs and events, for `/metrics`.
pub(crate) fn journal() -> &'static Journal {
    &JOURNAL
}

/// The `kind` of a job launched by `launch_proof`.
pub(crate) fn job_kind(execution_only: bool) -> &'static str {
    if execution_only {
        KIND_EXECUTE
    } else {
        KIND_PROVE
    }
}

/// `coordinator_worker_state` code of a worker: 0 disconnected, 1 ready, 2 setting up (or
/// connected without a setup), 3 computing, 4 recovering from a cancelled or failed job, 5 error.
pub(crate) fn worker_state_code(state: &WorkerState, recovering: bool) -> u8 {
    match state {
        WorkerState::Disconnected | WorkerState::Connecting => 0,
        WorkerState::Ready => 1,
        WorkerState::SettingUp if recovering => 4,
        WorkerState::Idle | WorkerState::SettingUp => 2,
        WorkerState::Computing(_) => 3,
        WorkerState::Error => 5,
    }
}

/// Record that a job has just been accepted and dispatched to `workers` workers. Pairs with
/// [`record_job_terminal`] (one increment ↔ one decrement of `coordinator_active_jobs`).
pub(crate) fn record_job_started(job_id: &JobId, kind: &'static str, workers: usize) {
    metrics::gauge!("coordinator_active_jobs").increment(1.0);
    journal().job_started(job_id, kind, workers);
}

/// A job in flight entered `phase` (one of the `PHASE_*` constants).
pub(crate) fn record_job_phase(job_id: &JobId, phase: &'static str) {
    journal().job_phase(job_id, phase);
}

/// A job in flight takes its input inline, `bytes` long.
pub(crate) fn record_job_input(job_id: &JobId, bytes: u64) {
    journal().job_input(job_id, bytes);
}

/// What a worker reported about running a job's guest.
pub(crate) struct WorkerExecution<'a> {
    /// The executor's own timing, in milliseconds, and the assembly execution's.
    pub executor: &'a ZiskExecutorTime,
    /// The witness computation, for a proof's phase 1.
    pub witness_seconds: Option<f64>,
    /// From the coordinator starting the phase to the worker receiving the task.
    pub dispatch_delay: Option<f64>,
}

/// A worker finished running a job's guest: phase 1 of a proof, or an execute-only job.
pub(crate) fn record_worker_execution(
    job_id: &JobId,
    kind: &'static str,
    report: WorkerExecution<'_>,
) {
    let executor = report.executor;
    if let Some(asm) = &executor.asm_execution_duration {
        let (time, mhz) = (f64::from(asm.time), f64::from(asm.mhz));
        metrics::gauge!("coordinator_last_asm_seconds", "kind" => kind).set(time);
        metrics::gauge!("coordinator_last_asm_mhz", "kind" => kind).set(mhz);
        let steps = time * mhz * 1e6;
        if steps.is_finite() && steps > 0.0 {
            journal().job_steps_estimate(job_id, steps as u64);
        }
    }
    for (part, ms) in [
        ("total", executor.total_duration),
        ("execution", executor.execution_duration),
        ("count_and_plan", executor.count_and_plan_duration),
        ("count_and_plan_mo", executor.count_and_plan_mo_duration),
    ] {
        metrics::gauge!("coordinator_last_executor_seconds", "kind" => kind, "part" => part)
            .set(ms as f64 / 1000.0);
    }
    if let Some(seconds) = report.witness_seconds {
        metrics::gauge!("coordinator_last_witness_seconds", "kind" => kind).set(seconds);
    }
    if let Some(seconds) = report.dispatch_delay {
        metrics::histogram!("coordinator_dispatch_delay_seconds", "kind" => kind).record(seconds);
        metrics::gauge!("coordinator_last_dispatch_delay_seconds", "kind" => kind).set(seconds);
    }
}

/// How long each worker took over phase `phase` ("1" contributions, "2" prove) of the proof
/// that just succeeded, from the phase's start to its result.
pub(crate) fn record_proof_worker_phases(phase: &'static str, durations: &[(WorkerId, f64)]) {
    for (worker_id, seconds) in durations {
        metrics::gauge!(
            "coordinator_last_proof_worker_phase_seconds",
            "phase" => phase, "worker_id" => worker_id.as_str().to_owned()
        )
        .set(*seconds);
    }
    if durations.len() > 1 {
        let slowest = durations.iter().map(|(_, s)| *s).fold(f64::MIN, f64::max);
        let fastest = durations.iter().map(|(_, s)| *s).fold(f64::MAX, f64::min);
        metrics::histogram!("coordinator_proof_worker_spread_seconds", "phase" => phase)
            .record(slowest - fastest);
    }
}

/// The instances per AIR of the last execution, from proofman's summary line.
pub(crate) fn record_execution_summary(summary: &str) {
    journal().execution_summary(summary);
}

/// How a job ended, for [`record_job_terminal`].
pub(crate) struct JobOutcome<'a> {
    pub job_id: &'a JobId,
    /// One of the `OUTCOME_*` constants.
    pub outcome: &'static str,
    /// Why a failed job failed.
    pub reason: Option<&'a str>,
    pub workers: &'a [WorkerId],
    /// When the Contributions (or Execution) phase started; `None` for jobs cancelled or failed
    /// before any phase began.
    pub contributions_started: Option<DateTime<Utc>>,
    /// Seconds from the job's first phase to its end, when known.
    pub duration: Option<f64>,
    /// A successful proof's three phases (contributions, prove, aggregation), in seconds.
    pub phases: Option<[f64; 3]>,
    pub steps: Option<u64>,
    pub instances: Option<u64>,
    /// The execution's cost per type, for a job that ran a guest.
    pub costs: Option<StatsCostPerType>,
}

/// Record metrics for a job that has reached a terminal state.
///
/// Emits in one call:
/// - decrement of `coordinator_active_jobs`
/// - increment of `coordinator_jobs_total{kind, outcome[, reason]}`
/// - per-worker increment of `coordinator_worker_jobs_total{worker_id, outcome=…}`
/// - end-to-end duration into `coordinator_job_duration_seconds{outcome=…}` if
///   the Contributions phase actually started (skipped for jobs cancelled/failed
///   before any phase began)
/// - for a success: the steps and the `coordinator_last_job_*` gauges, and for a proof its
///   phases (`coordinator_proof_phase_seconds`, `coordinator_last_proof_phase_seconds`)
/// - the journal's record of the job
pub(crate) fn record_job_terminal(o: JobOutcome<'_>) {
    let kind = journal().job_kind(o.job_id).unwrap_or(KIND_UNKNOWN);
    match (o.outcome, o.reason) {
        (OUTCOME_FAILURE, Some(reason)) => metrics::counter!(
            "coordinator_jobs_total",
            "kind" => kind, "outcome" => o.outcome, "reason" => failure_class(reason)
        )
        .increment(1),
        _ => metrics::counter!("coordinator_jobs_total", "kind" => kind, "outcome" => o.outcome)
            .increment(1),
    }
    metrics::gauge!("coordinator_active_jobs").decrement(1.0);

    // WorkerId::Display truncates + wraps in "WorkerId(...)" — wrong for label
    // values; use the raw UUID via as_str().
    for w in o.workers {
        metrics::counter!(
            "coordinator_worker_jobs_total",
            "worker_id" => w.as_str().to_owned(), "outcome" => o.outcome
        )
        .increment(1);
    }

    if let Some(started) = o.contributions_started {
        let elapsed = (Utc::now() - started).num_milliseconds() as f64 / 1000.0;
        metrics::histogram!(
            "coordinator_job_duration_seconds", "outcome" => o.outcome
        )
        .record(elapsed);
    }

    let mut cold = false;
    if o.outcome == OUTCOME_SUCCESS {
        if let Some(steps) = o.steps {
            metrics::counter!("coordinator_job_steps_total", "kind" => kind).increment(steps);
            metrics::gauge!("coordinator_last_job_steps", "kind" => kind).set(steps as f64);
        }
        if let Some(instances) = o.instances {
            metrics::gauge!("coordinator_last_job_instances", "kind" => kind).set(instances as f64);
        }
        if let Some(duration) = o.duration {
            metrics::gauge!("coordinator_last_job_duration_seconds", "kind" => kind).set(duration);
            if let Some(steps) = o.steps.filter(|_| duration > 0.0) {
                metrics::gauge!("coordinator_last_job_mhz", "kind" => kind)
                    .set(steps as f64 / duration / 1e6);
            }
        }
        if let Some(phases) = o.phases {
            cold = journal().take_cold();
            let cold_label = if cold { "true" } else { "false" };
            for (phase, seconds) in ["1", "2", "3"].into_iter().zip(phases) {
                metrics::histogram!(
                    "coordinator_proof_phase_seconds", "phase" => phase, "cold" => cold_label
                )
                .record(seconds);
                metrics::gauge!("coordinator_last_proof_phase_seconds", "phase" => phase)
                    .set(seconds);
            }
            metrics::gauge!("coordinator_last_job_cold", "kind" => kind).set(if cold {
                1.0
            } else {
                0.0
            });
        }
        if let Some(costs) = &o.costs {
            let split = [
                costs.main_cost,
                costs.opcode_cost,
                costs.memory_cost,
                costs.precompile_cost,
                costs.tables_cost,
                costs.other_cost,
            ];
            for (cost_type, cost) in COST_TYPES.into_iter().zip(split) {
                metrics::counter!("coordinator_job_cost_total", "kind" => kind, "type" => cost_type)
                    .increment(cost);
                metrics::gauge!("coordinator_last_job_cost", "kind" => kind, "type" => cost_type)
                    .set(cost as f64);
            }
        }
    }

    journal().job_ended(&JobEnded {
        job_id: o.job_id,
        kind,
        outcome: o.outcome,
        duration: o.duration,
        phases: o.phases,
        steps: o.steps,
        instances: o.instances,
        reason: o.reason,
        cold,
    });
}

/// The class of a failure reason: a phase or execution `timeout`, a `worker` error, or `other`.
fn failure_class(reason: &str) -> &'static str {
    let [timeout, worker, other] = FAILURE_CLASSES;
    let reason = reason.to_ascii_lowercase();
    if reason.contains("timed out") {
        timeout
    } else if reason.contains("worker") {
        worker
    } else {
        other
    }
}

/// A worker acknowledged a program setup, `seconds` after it was sent the setup if it was one
/// of the setup's workers.
pub(crate) fn record_setup_ack(
    worker_id: &WorkerId,
    hash_id: &str,
    success: bool,
    error: Option<&str>,
    seconds: Option<f64>,
) {
    let outcome = if success { OUTCOME_SUCCESS } else { OUTCOME_FAILURE };
    metrics::counter!("coordinator_setups_total", "outcome" => outcome).increment(1);
    let program = short_hash(hash_id);
    if success {
        let took = match seconds {
            Some(seconds) => {
                metrics::histogram!("coordinator_setup_seconds").record(seconds);
                metrics::gauge!("coordinator_last_setup_seconds", "hash_id" => program.clone())
                    .set(seconds);
                format!(" in {seconds:.1} s")
            }
            None => String::new(),
        };
        journal().event("setup done", format!("hash_id {program}… on {worker_id}{took}"));
    } else {
        let error = error.unwrap_or("unknown error");
        journal().event("setup failed", format!("hash_id {program}… on {worker_id}: {error}"));
    }
}

/// A program's hash as labels show it: its first 12 characters.
fn short_hash(hash_id: &str) -> String {
    hash_id.chars().take(12).collect()
}

/// A worker registered (`reconnected` when it was known already). `total`, `cc` and `acc` are
/// the pool's workers, compute units and available compute units after it.
pub(crate) fn record_worker_registered(
    worker_id: &WorkerId,
    reconnected: bool,
    pool: (usize, u32, u32),
) {
    let reconnect = if reconnected { "true" } else { "false" };
    metrics::counter!("coordinator_worker_registrations_total", "reconnect" => reconnect)
        .increment(1);
    journal().mark_cold();
    let kind = if reconnected { "worker reconnected" } else { "worker registered" };
    journal().event(kind, pool_detail(worker_id, pool));
}

/// A worker disconnected; `pool` as in [`record_worker_registered`].
pub(crate) fn record_worker_disconnected(worker_id: &WorkerId, pool: (usize, u32, u32)) {
    metrics::counter!("coordinator_worker_disconnects_total").increment(1);
    journal().event("worker disconnected", pool_detail(worker_id, pool));
}

fn pool_detail(worker_id: &WorkerId, (total, cc, acc): (usize, u32, u32)) -> String {
    format!("{worker_id} · {total} worker(s) · {cc} CU total, {acc} CU available")
}

/// A worker parked after a cancelled or failed job reported it is ready again, `seconds` after
/// it was parked.
pub(crate) fn record_worker_recovered(worker_id: &WorkerId, seconds: f64) {
    metrics::counter!("coordinator_worker_recoveries_total", "result" => "done").increment(1);
    metrics::histogram!("coordinator_worker_recovery_seconds").record(seconds);
    metrics::gauge!("coordinator_last_worker_recovery_seconds").set(seconds);
    journal().event("recovery done", format!("{worker_id} in {seconds:.2} s"));
}

/// A parked worker never reported its recovery and was given up on after `seconds`.
pub(crate) fn record_worker_recovery_expired(worker_id: &WorkerId, seconds: f64) {
    metrics::counter!("coordinator_worker_recoveries_total", "result" => "expired").increment(1);
    journal().event("recovery expired", format!("{worker_id} after {seconds:.0} s"));
}

/// A new program's ELF entered the coordinator's cache.
pub(crate) fn record_program_registered(hash_id: &str, bytes: usize) {
    let program = short_hash(hash_id);
    journal().event("program registered", format!("hash_id {program}… ({bytes} bytes)"));
}

/// The coordinator started. Its counters start at zero for every label set they can take, bar
/// the per-worker and per-location ones: a series that appears at its first event loses that
/// event to `increase()` and `rate()`, which need a sample before it.
pub(crate) fn record_coordinator_started() {
    for kind in [KIND_PROVE, KIND_EXECUTE, KIND_WRAP] {
        for outcome in [OUTCOME_SUCCESS, OUTCOME_CANCELLED] {
            metrics::counter!("coordinator_jobs_total", "kind" => kind, "outcome" => outcome)
                .increment(0);
        }
        for reason in FAILURE_CLASSES {
            metrics::counter!(
                "coordinator_jobs_total",
                "kind" => kind, "outcome" => OUTCOME_FAILURE, "reason" => reason
            )
            .increment(0);
        }
        metrics::counter!("coordinator_job_steps_total", "kind" => kind).increment(0);
        for cost_type in COST_TYPES {
            metrics::counter!("coordinator_job_cost_total", "kind" => kind, "type" => cost_type)
                .increment(0);
        }
    }
    for outcome in [OUTCOME_SUCCESS, OUTCOME_FAILURE] {
        metrics::counter!("coordinator_setups_total", "outcome" => outcome).increment(0);
    }
    for reconnect in ["false", "true"] {
        metrics::counter!("coordinator_worker_registrations_total", "reconnect" => reconnect)
            .increment(0);
    }
    metrics::counter!("coordinator_worker_disconnects_total").increment(0);
    for result in ["done", "expired"] {
        metrics::counter!("coordinator_worker_recoveries_total", "result" => result).increment(0);
    }
    worker_report::zero_exit_counters();
    journal().event("coordinator started", format!("version {}", env!("CARGO_PKG_VERSION")));
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_cluster_common::JobPhase;

    #[test]
    fn a_hash_from_a_worker_is_shortened_by_characters_not_bytes() {
        assert_eq!(short_hash("0123456789abcdef"), "0123456789ab");
        assert_eq!(short_hash("ééééééééééééé"), "éééééééééééé");
        // Must not panic on a multi-byte character at the 12-byte boundary.
        record_setup_ack(&WorkerId::from("w".to_string()), "aaaaaaaaaaaé", true, None, Some(1.0));
    }

    #[test]
    fn failure_reasons_fall_into_three_classes() {
        assert_eq!(
            failure_class("[Monitor] Phase Recurse timed out for job x (107s > 100s)"),
            "timeout"
        );
        assert_eq!(
            failure_class("Task execution failed on worker WorkerId(1): Error aggregating proofs"),
            "worker"
        );
        assert_eq!(failure_class("Failed to dispatch Phase 1"), "other");
    }

    #[test]
    fn a_parked_worker_reads_as_recovering_and_a_setting_up_one_does_not() {
        assert_eq!(worker_state_code(&WorkerState::SettingUp, true), 4);
        assert_eq!(worker_state_code(&WorkerState::SettingUp, false), 2);
        assert_eq!(worker_state_code(&WorkerState::Idle, false), 2);
        assert_eq!(worker_state_code(&WorkerState::Ready, false), 1);
        let job = JobId::new();
        assert_eq!(worker_state_code(&WorkerState::Computing((job, JobPhase::Prove)), false), 3);
        assert_eq!(worker_state_code(&WorkerState::Disconnected, true), 0);
        assert_eq!(worker_state_code(&WorkerState::Error, false), 5);
    }
}
