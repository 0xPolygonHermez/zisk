//! Centralised emission of coordinator job-lifecycle metrics.
//!
//! All metric *descriptions* live in `zisk-coordinator-server`'s `metrics` module
//! (alongside the Prometheus recorder install). This module only emits values.
//! One-shot gauge updates that don't share semantics (program registration,
//! worker pool +/-) stay inline at their callsites.

use chrono::{DateTime, Utc};
use zisk_cluster_common::WorkerId;

/// Closed set of `outcome` label values. Sharing the constants between the
/// helper and its callers keeps the label cardinality fixed and avoids drift
/// against the Prometheus descriptions in `zisk-coordinator-server`.
pub(crate) const OUTCOME_SUCCESS: &str = "success";
pub(crate) const OUTCOME_FAILURE: &str = "failure";
pub(crate) const OUTCOME_CANCELLED: &str = "cancelled";

/// Record that a job has just been accepted and dispatched. Pairs with
/// `record_job_terminal` (one increment ↔ one decrement of `coordinator_active_jobs`).
pub(crate) fn record_job_started() {
    metrics::gauge!("coordinator_active_jobs").increment(1.0);
}

/// Record metrics for a job that has reached a terminal state.
///
/// Emits in one call:
/// - decrement of `coordinator_active_jobs`
/// - increment of `coordinator_jobs_total{kind="prove", outcome=…}`
/// - per-worker increment of `coordinator_worker_jobs_total{worker_id, outcome=…}`
/// - end-to-end duration into `coordinator_job_duration_seconds{outcome=…}` if
///   the Contributions phase actually started (skipped for jobs cancelled/failed
///   before any phase began).
pub(crate) fn record_job_terminal(
    outcome: &'static str,
    workers: &[WorkerId],
    contributions_started: Option<DateTime<Utc>>,
) {
    metrics::counter!(
        "coordinator_jobs_total", "kind" => "prove", "outcome" => outcome
    )
    .increment(1);
    metrics::gauge!("coordinator_active_jobs").decrement(1.0);

    // WorkerId::Display truncates + wraps in "WorkerId(...)" — wrong for label
    // values; use the raw UUID via as_str().
    for w in workers {
        metrics::counter!(
            "coordinator_worker_jobs_total",
            "worker_id" => w.as_str().to_owned(), "outcome" => outcome
        )
        .increment(1);
    }

    if let Some(started) = contributions_started {
        let elapsed = (Utc::now() - started).num_milliseconds() as f64 / 1000.0;
        metrics::histogram!(
            "coordinator_job_duration_seconds", "outcome" => outcome
        )
        .record(elapsed);
    }
}
