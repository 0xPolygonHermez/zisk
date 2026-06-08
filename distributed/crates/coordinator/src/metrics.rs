//! Centralised emission of coordinator job-lifecycle metrics.
//!
//! All metric *descriptions* live in `zisk-coordinator-server`'s `metrics` module
//! (alongside the Prometheus recorder install). This module only emits values.
//! One-shot gauge updates that don't share semantics (program registration,
//! worker pool +/-) stay inline at their callsites.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use zisk_cluster_common::{JobPhase, PhaseTimings, WorkerId};

/// Closed set of `outcome` label values. Sharing the constants between the
/// helper and its callers keeps the label cardinality fixed and avoids drift
/// against the Prometheus descriptions in `zisk-coordinator-server`.
pub(crate) const OUTCOME_SUCCESS: &str = "success";
pub(crate) const OUTCOME_FAILURE: &str = "failure";
pub(crate) const OUTCOME_CANCELLED: &str = "cancelled";

/// Closed set of `kind` label values. Jobs with `execution_only=true` skip the
/// proof pipeline and must not be hidden inside proof counters.
pub(crate) const KIND_PROVE: &str = "prove";
pub(crate) const KIND_EXECUTE: &str = "execute";

pub(crate) fn job_kind(execution_only: bool) -> &'static str {
    if execution_only {
        KIND_EXECUTE
    } else {
        KIND_PROVE
    }
}

fn classify_failure_reason(reason: &str) -> &'static str {
    let reason = reason.to_ascii_lowercase();
    if reason.contains("timed out") || reason.contains("timeout") {
        "phase_timeout"
    } else if reason.contains("heartbeat") || reason.contains("stale") {
        "stale_heartbeat"
    } else if reason.contains("channel closed") || reason.contains("send message") {
        "worker_channel"
    } else if reason.contains("unreachable") {
        "worker_unreachable"
    } else if reason.contains("setup") {
        "setup_failed"
    } else {
        "unknown"
    }
}

/// Closed-set worker error taxonomy. Used as the `reason` label value for
/// `coordinator_worker_errors_total`; keep it small to bound cardinality.
pub(crate) const WORKER_ERROR_HEARTBEAT_LOST: &str = "heartbeat_lost";
pub(crate) const WORKER_ERROR_CHANNEL_CLOSED: &str = "channel_closed";
pub(crate) const WORKER_ERROR_SETUP_FAIL: &str = "setup_fail";
pub(crate) const WORKER_ERROR_PROVE_FAIL: &str = "prove_fail";
pub(crate) const WORKER_ERROR_AGG_FAIL: &str = "agg_fail";
pub(crate) const WORKER_ERROR_UNREACHABLE: &str = "unreachable";
pub(crate) const WORKER_ERROR_UNKNOWN: &str = "unknown";

/// Map a free-form worker error string to the bounded taxonomy emitted via
/// the `coordinator_worker_errors_total` counter.
///
/// The raw text is intentionally not used as a Prometheus label because it is
/// effectively unbounded; high-cardinality details belong in the JSON/Postgres
/// history path.
pub(crate) fn classify_worker_error(reason: &str) -> &'static str {
    let reason = reason.to_ascii_lowercase();
    if reason.contains("heartbeat") || reason.contains("stale") {
        WORKER_ERROR_HEARTBEAT_LOST
    } else if reason.contains("channel closed") {
        WORKER_ERROR_CHANNEL_CLOSED
    } else if reason.contains("unreachable") {
        WORKER_ERROR_UNREACHABLE
    } else if reason.contains("setup") {
        WORKER_ERROR_SETUP_FAIL
    } else if reason.contains("aggregate") || reason.contains("agg ") {
        WORKER_ERROR_AGG_FAIL
    } else if reason.contains("prove") || reason.contains("phase 2") {
        WORKER_ERROR_PROVE_FAIL
    } else if reason.contains("send message") || reason.contains("send_message") {
        WORKER_ERROR_CHANNEL_CLOSED
    } else {
        WORKER_ERROR_UNKNOWN
    }
}

/// Increment `coordinator_worker_errors_total` for a single worker-scoped
/// failure event. `reason` should be one of the `WORKER_ERROR_*` constants
/// (typically produced via [`classify_worker_error`]).
pub(crate) fn record_worker_error(worker_id: &str, program: &str, reason: &str) {
    metrics::counter!(
        "coordinator_worker_errors_total",
        "worker_id" => worker_id.to_owned(),
        "program" => program.to_owned(),
        "reason" => reason.to_owned()
    )
    .increment(1);
}

/// Record a bounded failure-reason taxonomy for failed proof jobs.
///
/// The raw error text is intentionally not used as a Prometheus label; it is
/// high-cardinality and belongs in the JSON/Postgres history path.
pub(crate) fn record_job_failure_reason(reason: &str, program: &str) {
    metrics::counter!(
        "coordinator_job_failures_total",
        "program" => program.to_owned(),
        "reason" => classify_failure_reason(reason)
    )
    .increment(1);
}

/// Record that a job has just been accepted and dispatched. Pairs with
/// `record_job_terminal` (one increment and one decrement of `coordinator_active_jobs`).
pub(crate) fn record_job_started(kind: &'static str, program: &str) {
    metrics::gauge!("coordinator_active_jobs", "kind" => kind, "program" => program.to_owned())
        .increment(1.0);
}

pub(crate) fn record_phase_durations(
    program: &str,
    phase_timings: &HashMap<JobPhase, PhaseTimings>,
) {
    for (phase, timing) in phase_timings {
        let Some(end) = timing.end_time else {
            continue;
        };
        let elapsed =
            end.signed_duration_since(timing.start_time).num_milliseconds().max(0) as f64 / 1000.0;
        metrics::histogram!(
            "coordinator_phase_duration_seconds",
            "phase" => phase.to_string(),
            "program" => program.to_owned()
        )
        .record(elapsed);
    }
}

/// Record metrics for a job that has reached a terminal state.
///
/// Emits in one call:
/// - decrement of `coordinator_active_jobs{kind, program}`
/// - increment of `coordinator_jobs_total{kind, program, outcome}`
/// - per-worker increment of `coordinator_worker_jobs_total{worker_id, program, outcome}`
/// - end-to-end duration into `coordinator_job_duration_seconds{kind, program, outcome}`
///   if the first compute phase actually started (skipped for jobs cancelled/failed
///   before any compute phase began).
pub(crate) fn record_job_terminal(
    kind: &'static str,
    outcome: &'static str,
    program: &str,
    workers: &[WorkerId],
    compute_started: Option<DateTime<Utc>>,
    executed_steps: Option<u64>,
) {
    metrics::counter!(
        "coordinator_jobs_total",
        "kind" => kind, "program" => program.to_owned(), "outcome" => outcome
    )
    .increment(1);
    if outcome == OUTCOME_SUCCESS {
        metrics::gauge!("coordinator_last_successful_job_timestamp_seconds")
            .set(Utc::now().timestamp() as f64);
    }
    metrics::gauge!("coordinator_active_jobs", "kind" => kind, "program" => program.to_owned())
        .decrement(1.0);

    // Use the raw UUID label; WorkerId::Display truncates and wraps the value.
    for w in workers {
        metrics::counter!(
            "coordinator_worker_jobs_total",
            "worker_id" => w.as_str().to_owned(),
            "program" => program.to_owned(),
            "outcome" => outcome
        )
        .increment(1);
    }

    if let Some(started) = compute_started {
        let elapsed = (Utc::now() - started).num_milliseconds() as f64 / 1000.0;
        metrics::histogram!(
            "coordinator_job_duration_seconds",
            "kind" => kind, "program" => program.to_owned(), "outcome" => outcome
        )
        .record(elapsed);
    }

    if let Some(steps) = executed_steps {
        metrics::counter!("coordinator_job_executed_steps_total", "program" => program.to_owned())
            .increment(steps);
    }
}

#[cfg(test)]
#[path = "../tests/unit/metrics_worker_error.rs"]
mod worker_error_tests;

#[cfg(test)]
mod tests {
    use super::{classify_failure_reason, job_kind, KIND_EXECUTE, KIND_PROVE};

    #[test]
    fn failure_reason_classifier_is_bounded() {
        assert_eq!(classify_failure_reason("Phase Aggregate timed out"), "phase_timeout");
        assert_eq!(classify_failure_reason("worker heartbeat stale"), "stale_heartbeat");
        assert_eq!(
            classify_failure_reason("Failed to send message: channel closed"),
            "worker_channel"
        );
        assert_eq!(
            classify_failure_reason("all workers unreachable during setup"),
            "worker_unreachable"
        );
        assert_eq!(classify_failure_reason("unexpected backend error 123"), "unknown");
    }

    #[test]
    fn job_kind_is_bounded() {
        assert_eq!(job_kind(false), KIND_PROVE);
        assert_eq!(job_kind(true), KIND_EXECUTE);
    }
}
