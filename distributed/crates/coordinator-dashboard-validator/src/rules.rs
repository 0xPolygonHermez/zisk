//! Static rule tables for dashboard validation.

pub const EXPECTED_DASHBOARD_UID: &str = "zisk-dev";

/// `(fragment, reason)` pairs. Iteration order matters for stable errors.
pub const BANNED_PROMQL_FRAGMENTS: &[(&str, &str)] = &[
    ("coordinator_worker_state", "v0.18 does not emit per-worker state"),
    (
        "worker_phase",
        "v0.18 coordinator dashboard does not scrape worker phase metrics",
    ),
    (
        "worker_state",
        "v0.18 coordinator dashboard does not scrape worker state metrics",
    ),
    (
        "worker_last_heartbeat",
        "v0.18 coordinator dashboard does not emit heartbeat gauges",
    ),
    (
        "coordinator_job_phase_duration_seconds",
        "v0.18 emits coordinator_phase_duration_seconds, not coordinator_job_phase_duration_seconds",
    ),
    ("job_id=", "job_id must stay out of Prometheus labels"),
    ("hash_id=", "hash_id must stay out of Prometheus labels"),
];

pub const BANNED_DASHBOARD_URL_FRAGMENTS: &[(&str, &str)] = &[
    ("127.0.0.1", "dashboard JSON must be environment-neutral"),
    ("localhost", "dashboard JSON must be environment-neutral"),
    ("http://127.0.0.1", "dashboard JSON must be environment-neutral"),
    ("http://localhost", "dashboard JSON must be environment-neutral"),
    (":9999/workers/", "v0.18 removed the old worker metrics proxy path"),
    ("/api/v1/live", "coordinator observability HTTP API does not implement /api/v1/live"),
    (
        "/api/v1/jobs/recent",
        "history panels must use the Postgres datasource, not deprecated coordinator JSON",
    ),
    (
        "/api/v1/jobs/phases/recent",
        "phase history panels must use the Postgres datasource, not deprecated coordinator JSON",
    ),
    (
        "/api/v1/jobs/stats/recent",
        "history statistics panels must use the Postgres datasource, not deprecated coordinator JSON",
    ),
    (
        "/api/v1/programs/performance",
        "program performance panels must use Prometheus or Postgres, not deprecated coordinator JSON",
    ),
    (
        "/api/v1/workers/errors/recent",
        "worker error history panels must use the Postgres datasource, not deprecated coordinator JSON",
    ),
    (
        "/api/v1/coordinators",
        "coordinator inventory belongs in Prometheus or tooling, not dashboard JSON",
    ),
];

pub const REQUIRED_PANEL_TITLES: &[&str] = &[
    "Coordinator Availability (now)",
    "Workers Connected (now)",
    "Active Proofs (now)",
    "Average Proof Duration (last 10 successes)",
    "Last Successful Proof Age (now)",
    "Infrastructure Health",
    "Current Proof",
    "Current Proof Phase (now)",
    "Current Proof Duration (now)",
    "Current Phase Age (now)",
    "Progress Update Age (now)",
    "Workers Assigned (now)",
    "Proof Phase Progress (latest jobs)",
    "Reliability",
    "Proof Success Rate (24h)",
    "Proof Duration Stats (24h)",
    "Proof Failure Rate by Kind (5m)",
    "Proof Failures by Reason (selected range)",
    "Proof Performance",
    "Proof Duration Distribution (recent jobs)",
    "Proof Duration Quantiles (24h)",
    "Proof Duration by Cost (all proofs)",
    "Performance Trends",
    "Stage Utilization by Phase (15m)",
    "Executed Cycles Rate by Program (15m)",
    "Proof Duration p95 by Program (15m)",
    "Phase Duration p95 by Program and Phase (15m)",
    "Program Performance Summary (24h)",
    "Worker Fleet",
    "Worker Heartbeat Lag by Worker",
    "Worker Roster",
    "Worker Diagnostics",
    "Worker Assignments by Worker",
    "Worker Error Events",
    "Coordinator Runtime",
    "Coordinator Availability Timeline",
    "Coordinator Restarts (selected range)",
    "Recent Proof History",
    "gRPC Request Rate by Status",
    "History Writer Queue and Drops",
    "History DB Latency p95 by Operation",
];

// Live Current Proof cards + Worker Roster excluded: coord JSON endpoints
// filter strictly (idle workers + non-matching program rows render empty),
// so passing $program blanks the cards during real activity.
pub const PROGRAM_PANELS: &[&str] = &[
    "Proof Phase Progress (latest jobs)",
    "Average Proof Duration (last 10 successes)",
    "Proof Duration Stats (24h)",
    "Proof Failures by Reason (selected range)",
    "Recent Proof History",
    "Proof Duration Distribution (recent jobs)",
    "Proof Duration Quantiles (24h)",
    "Proof Duration by Cost (all proofs)",
    "Stage Utilization by Phase (15m)",
    "Executed Cycles Rate by Program (15m)",
    "Program Performance Summary (24h)",
];

pub const POSTGRES_HISTORY_PANELS: &[&str] = &[
    "Average Proof Duration (last 10 successes)",
    "Proof Phase Progress (latest jobs)",
    "Proof Duration Stats (24h)",
    "Proof Failures by Reason (selected range)",
    "Recent Proof History",
    "Proof Duration Distribution (recent jobs)",
    "Proof Duration Quantiles (24h)",
    "Proof Duration by Cost (all proofs)",
    "Program Performance Summary (24h)",
    "Worker Error Events",
];

pub const HISTOGRAM_SNAPSHOT_PANELS: &[(&str, &str)] =
    &[("Proof Duration Distribution (recent jobs)", "duration_ms")];

pub const PROMETHEUS_TREND_PANELS: &[(&str, &str)] = &[
    ("Stage Utilization by Phase (15m)", "coordinator_phase_duration_seconds_sum"),
    ("Executed Cycles Rate by Program (15m)", "coordinator_job_executed_steps_total"),
];

pub const PROMETHEUS_TOP_ROW: &[&str] = &[
    "Coordinator Availability (now)",
    "Active Proofs (now)",
    "Workers Connected (now)",
    "Last Successful Proof Age (now)",
];

// Cards inside the expanded "Current Proof" row.
pub const PROOF_RUN_SUMMARY_CARDS: &[(&str, &str)] = &[
    ("Current Proof Phase (now)", "phase_code"),
    ("Current Proof Duration (now)", "age_seconds"),
    ("Current Phase Age (now)", "phase_age_seconds"),
    ("Progress Update Age (now)", "update_age_seconds"),
    ("Workers Assigned (now)", "workers_count"),
];

/// Panels that must query their expected metric token.
pub const REQUIRED_METRIC_PANELS: &[(&str, &str)] = &[
    ("Proof Duration p95 by Program (15m)", "coordinator_job_duration_seconds_bucket"),
    ("Phase Duration p95 by Program and Phase (15m)", "coordinator_phase_duration_seconds_bucket"),
    ("Worker Heartbeat Lag by Worker", "coordinator_worker_heartbeat_lag_seconds"),
    ("Coordinator Availability Timeline", "up{"),
    ("Coordinator Restarts (selected range)", "coordinator_start_time_seconds"),
];

/// PromQL functions that are not metric names.
pub const PROMQL_FUNCTIONS: &[&str] = &[
    "abs",
    "avg",
    "avg_over_time",
    "changes",
    "clamp_max",
    "clamp_min",
    "count",
    "histogram_quantile",
    "increase",
    "last_over_time",
    "max",
    "max_over_time",
    "min",
    "or",
    "rate",
    "sum",
    "time",
    "vector",
];
