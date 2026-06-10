//! Prometheus metrics registration and HTTP scrape endpoint.
//!
//! ## Exposed metrics
//!
//! | Name | Type | Labels | Description |
//! |------|------|--------|-------------|
//! | `coordinator_info` | Gauge | `coordinator_id`, `version`, `environment` | Constant 1 per running coordinator |
//! | `coordinator_start_time_seconds` | Gauge | `coordinator_id` | Coordinator process start time as Unix seconds |
//! | `coordinator_requests_total` | Counter | `coordinator_id`, `method`, `status` | gRPC calls by method and outcome |
//! | `coordinator_request_duration_seconds` | Histogram | `coordinator_id`, `method` | gRPC call latency |
//! | `coordinator_active_jobs` | Gauge | `coordinator_id`, `kind`, `program` | Currently active (non-terminal) jobs |
//! | `coordinator_jobs_total` | Counter | `coordinator_id`, `kind`, `outcome`, `program` | Jobs by kind, program, and final outcome |
//! | `coordinator_job_failures_total` | Counter | `coordinator_id`, `reason`, `program` | Failed proof jobs by bounded failure taxonomy |
//! | `coordinator_last_successful_job_timestamp_seconds` | Gauge | `coordinator_id` | Unix timestamp for the latest successful proof job |
//! | `coordinator_registered_programs_total` | Counter | `coordinator_id`, `program` | Guest programs newly cached since process start |
//! | `coordinator_workers_connected` | Gauge | `coordinator_id` | Workers currently registered in the pool |
//! | `coordinator_workers_by_status` | Gauge | `coordinator_id`, `status` | Workers grouped by coordinator-owned lifecycle state |
//! | `coordinator_worker_jobs_total` | Counter | `coordinator_id`, `worker_id`, `program`, `outcome` | Per-worker participation count by job outcome |
//! | `coordinator_worker_errors_total` | Counter | `coordinator_id`, `worker_id`, `program`, `reason` | Worker-scoped failure taxonomy when available |
//! | `coordinator_job_duration_seconds` | Histogram | `coordinator_id`, `kind`, `outcome`, `program` | End-to-end job duration (first compute phase start to terminal state) |
//! | `coordinator_phase_duration_seconds` | Histogram | `coordinator_id`, `phase`, `program` | Per-phase proof duration |
//! | `coordinator_job_executed_steps_total` | Counter | `coordinator_id`, `program` | Executed zkVM steps for completed jobs |
//! | `coordinator_program_info` | Gauge | `coordinator_id`, `program`, `hash_id` | Constant 1 for join between bounded program alias and raw guest program hash |
//! | `coordinator_restarts_total` | Counter | `coordinator_id` | Coordinator process restarts since metric inception (incremented once per construction) |
//! | `coordinator_worker_heartbeat_lag_seconds` | Gauge | `coordinator_id`, `worker_id` | Seconds since this worker's last heartbeat reached the coordinator |
//! | `coordinator_deprecated_endpoint_hits_total` | Counter | `coordinator_id`, `path` | Compatibility REST endpoint usage by normalized path |
//!
//! ## Scrape endpoint
//!
//! When metrics are enabled, a lightweight HTTP server is started on
//! `metrics.host:metrics.port` (default `0.0.0.0:9090`).
//!
//! - `GET /metrics` returns Prometheus text format
//! - `GET /health` returns `200 OK` (liveness probe)
//! - `GET /api/v1/jobs/current` returns one-row current proof status for dashboard
//!   hero cards, sourced from live coordinator state
//! - `GET /api/v1/workers` returns live worker roster, sourced from live coordinator
//!   state
//!
//! History-style JSON routes such as `/api/v1/jobs/recent` remain as
//! compatibility endpoints only. New dashboards should use the provisioned
//! Postgres datasource for historical views and the live JSON routes above for
//! current process state.

use std::{
    borrow::Cow,
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;
use zisk_coordinator::{
    JobHistoryJob, JobHistoryListQuery, JobHistoryStore, JobHistoryWorkerError,
    ProgramPerformancePage, WorkerErrorQuery,
};

use crate::backend::{LiveJobSnapshot, LiveStateProvider};
use crate::config::MetricsConfig;

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
static COORDINATOR_STARTED_AT: OnceLock<DateTime<Utc>> = OnceLock::new();
/// Cached copies of the coordinator identity labels so the JSON
/// `/api/v1/coordinators` route can return them without re-reading the config
/// at request time. Set once during `install_prometheus` /
/// `record_coordinator_info`.
static COORDINATOR_ID: OnceLock<String> = OnceLock::new();
static COORDINATOR_VERSION: OnceLock<String> = OnceLock::new();
static COORDINATOR_ENVIRONMENT: OnceLock<String> = OnceLock::new();
static DEPRECATED_ENDPOINT_WARN_CACHE: OnceLock<
    Mutex<HashMap<DeprecatedEndpointWarnKey, DateTime<Utc>>>,
> = OnceLock::new();
const WORKER_STATUS_LABELS: [&str; 7] =
    ["ready", "idle", "setting_up", "running", "disconnected", "connecting", "error"];

const DEPRECATED_REST_SUNSET: &str = "Wed, 30 Sep 2026 00:00:00 GMT";
const DEPRECATED_REST_LINK: &str =
    "</api/v1/openapi.json>; rel=\"deprecation\"; type=\"application/json\"";
const UNKNOWN_REQUEST_METADATA: &str = "unknown";

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct DeprecatedEndpointWarnKey {
    path: String,
    peer: String,
    user_agent: String,
}

#[derive(Debug)]
struct RequestContext {
    peer: String,
    user_agent: String,
}

#[derive(Debug, Serialize)]
struct PhaseLifecyclePage {
    data: Vec<PhaseLifecycleRow>,
}

#[derive(Debug, Serialize)]
struct PhaseLifecycleRow {
    lane: String,
    coordinator_id: String,
    job_id: String,
    job_label: String,
    program: String,
    state: String,
    phase: String,
    time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
    duration_ms: Option<u64>,
    workers_count: usize,
    current: bool,
}

#[derive(Debug, Serialize)]
struct JobHistoryStatsResponse {
    data: Vec<JobHistoryStatsSummary>,
    outcomes: Vec<JobHistoryOutcomeStats>,
}

#[derive(Debug, Serialize)]
struct JobHistoryStatsSummary {
    window_seconds: Option<i64>,
    sample_limit: usize,
    terminal_jobs: usize,
    success_count: usize,
    failure_count: usize,
    cancelled_count: usize,
    avg_duration_ms: Option<u64>,
    p50_duration_ms: Option<u64>,
    p95_duration_ms: Option<u64>,
    p99_duration_ms: Option<u64>,
    max_duration_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct JobHistoryOutcomeStats {
    outcome: String,
    jobs: usize,
    avg_duration_ms: Option<u64>,
    p50_duration_ms: Option<u64>,
    p95_duration_ms: Option<u64>,
    p99_duration_ms: Option<u64>,
    max_duration_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct CurrentJobStatusResponse {
    data: Vec<CurrentJobStatusRow>,
}

#[derive(Debug, Serialize)]
struct CurrentJobStatusRow {
    status: String,
    phase: String,
    phase_code: u8,
    coordinator_id: Option<String>,
    job_id: Option<String>,
    state: Option<String>,
    age_seconds: Option<u64>,
    phase_age_seconds: Option<u64>,
    update_age_seconds: Option<u64>,
    workers_count: Option<usize>,
}

#[derive(Debug, Serialize)]
struct WorkersPage<T> {
    data: Vec<T>,
}

#[derive(Debug, Serialize)]
struct CoordinatorMetadataRow {
    coordinator_id: String,
    environment: String,
    version: String,
    started_at: Option<DateTime<Utc>>,
    last_seen_at: DateTime<Utc>,
    up: bool,
}

#[derive(Debug, Serialize)]
struct RecentWorkerErrorRow {
    worker_id: String,
    job_id: Option<String>,
    program: String,
    reason: String,
    message: String,
    occurred_at: DateTime<Utc>,
}

/// Install the Prometheus global recorder and stash the handle for `/metrics`
/// rendering. Must be called once at startup before any metric is recorded.
pub fn install_prometheus(coordinator_id: &str) -> Result<()> {
    // Proofs commonly run for minutes, so duration buckets must cover the
    // operator-facing 30s to 30min range instead of generic request latency.
    const PROOF_DURATION_BUCKETS: &[f64] = &[
        10.0, 30.0, 60.0, 120.0, 240.0, 360.0, 480.0, 600.0, 900.0, 1200.0, 1800.0, 2700.0, 3600.0,
    ];
    const PHASE_DURATION_BUCKETS: &[f64] =
        &[1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 240.0, 360.0, 480.0, 600.0, 900.0, 1200.0, 1800.0];

    let handle = PrometheusBuilder::new()
        .add_global_label("coordinator_id", coordinator_id.to_owned())
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "coordinator_job_duration_seconds".to_owned(),
            ),
            PROOF_DURATION_BUCKETS,
        )?
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "coordinator_phase_duration_seconds".to_owned(),
            ),
            PHASE_DURATION_BUCKETS,
        )?
        .install_recorder()?;
    PROMETHEUS_HANDLE.set(handle).ok(); // ignore if already set (e.g. in tests)
    let _ = COORDINATOR_ID.set(coordinator_id.to_owned());
    register_descriptions();
    seed_job_kind_metrics();
    seed_worker_metrics();
    Ok(())
}

pub fn record_coordinator_info(version: &str, environment: &str) {
    metrics::gauge!(
        "coordinator_info",
        "version" => version.to_owned(),
        "environment" => environment.to_owned()
    )
    .set(1.0);
    let _ = COORDINATOR_VERSION.set(version.to_owned());
    let _ = COORDINATOR_ENVIRONMENT.set(environment.to_owned());
}

pub fn record_coordinator_start_time(started_at: DateTime<Utc>) {
    let _ = COORDINATOR_STARTED_AT.set(started_at);
    metrics::gauge!("coordinator_start_time_seconds").set(started_at.timestamp() as f64);
}

pub fn record_last_successful_job_timestamp(timestamp: Option<DateTime<Utc>>) {
    let value = timestamp.map_or(0.0, |ts| ts.timestamp() as f64);
    metrics::gauge!("coordinator_last_successful_job_timestamp_seconds").set(value);
}

fn register_descriptions() {
    metrics::describe_gauge!(
        "coordinator_info",
        "Constant 1 per running coordinator; carries version/environment labels for joins"
    );
    metrics::describe_gauge!(
        "coordinator_start_time_seconds",
        "Coordinator process start time as Unix seconds; use changes() for restart annotations"
    );
    metrics::describe_counter!(
        "coordinator_requests_total",
        "Total gRPC calls by method and outcome (ok/error)"
    );
    metrics::describe_histogram!(
        "coordinator_request_duration_seconds",
        "gRPC call latency in seconds"
    );
    metrics::describe_gauge!(
        "coordinator_active_jobs",
        "Number of currently active (non-terminal) jobs, labelled by kind and bounded program alias"
    );
    metrics::describe_counter!(
        "coordinator_jobs_total",
        "Jobs submitted, labelled by kind, final outcome, and bounded program alias"
    );
    metrics::describe_counter!(
        "coordinator_job_failures_total",
        "Failed proof jobs labelled by bounded reason taxonomy and program alias"
    );
    metrics::describe_gauge!(
        "coordinator_last_successful_job_timestamp_seconds",
        "Unix timestamp for the latest successful proof job"
    );
    metrics::describe_counter!(
        "coordinator_registered_programs_total",
        "Guest programs newly cached since coordinator process start, labelled by bounded program alias"
    );
    metrics::describe_gauge!(
        "coordinator_workers_connected",
        "Number of workers currently registered in the coordinator's pool"
    );
    metrics::describe_gauge!(
        "coordinator_workers_by_status",
        "Workers grouped by coordinator-owned lifecycle status"
    );
    metrics::describe_counter!(
        "coordinator_worker_jobs_total",
        "Per-worker participation count, labelled by worker_id, program alias, and final outcome"
    );
    metrics::describe_counter!(
        "coordinator_worker_errors_total",
        "Worker-scoped error taxonomy, labelled by worker, bounded program alias, and bounded reason"
    );
    metrics::describe_histogram!(
        "coordinator_job_duration_seconds",
        "End-to-end job duration (first compute phase start to terminal state) in seconds"
    );
    metrics::describe_histogram!(
        "coordinator_phase_duration_seconds",
        "Per-phase proof duration in seconds, labelled by phase and bounded program alias"
    );
    metrics::describe_counter!(
        "coordinator_job_executed_steps_total",
        "Executed zkVM steps for completed jobs, labelled by bounded program alias"
    );
    metrics::describe_gauge!(
        "coordinator_program_info",
        "Constant 1 per registered guest program; carries bounded program alias and the raw hash_id for joins between dashboards and history"
    );
    metrics::describe_counter!(
        "coordinator_restarts_total",
        "Coordinator process restarts. Incremented exactly once per coordinator construction so increase() on a 5m window surfaces unexpected restarts"
    );
    metrics::describe_gauge!(
        "coordinator_worker_heartbeat_lag_seconds",
        "Seconds since the last heartbeat for this worker reached the coordinator. Refreshed on every heartbeat and on each monitor sweep"
    );
    metrics::describe_gauge!(
        "coordinator_db_write_queue_depth",
        "Number of history events queued for the Postgres writer, sampled on every enqueue and drain"
    );
    metrics::describe_counter!(
        "coordinator_db_write_dropped_total",
        "History writes dropped before durable Postgres persistence, labelled by event_type"
    );
    metrics::describe_histogram!(
        "coordinator_db_query_duration_seconds",
        "Postgres query latency in seconds, labelled by op and status"
    );
    metrics::describe_gauge!(
        "coordinator_db_pool_size",
        "Postgres connection pool size, labelled by state (active or idle). Sampled on every pool acquire and release"
    );
    metrics::describe_counter!(
        "coordinator_deprecated_endpoint_hits_total",
        "Compatibility coordinator REST endpoint usage, labelled by normalized path. Use this during the sunset period before deleting history-style JSON routes"
    );
}

fn seed_job_kind_metrics() {
    for kind in ["prove", "execute"] {
        metrics::gauge!("coordinator_active_jobs", "kind" => kind, "program" => "unknown").set(0.0);
    }
}

fn seed_worker_metrics() {
    metrics::gauge!("coordinator_workers_connected").set(0.0);
    for status in WORKER_STATUS_LABELS {
        metrics::gauge!("coordinator_workers_by_status", "status" => status).set(0.0);
    }
}

/// Start the HTTP server that serves `/metrics` and `/health`.
/// No-ops if `cfg.enabled` is false.
/// The server stops accepting new connections when `cancel` is cancelled.
pub async fn start(
    cfg: &MetricsConfig,
    cancel: CancellationToken,
    history: Option<Arc<dyn JobHistoryStore>>,
    live_state: Option<Arc<dyn LiveStateProvider>>,
) -> Result<()> {
    if !cfg.enabled {
        return Ok(());
    }

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            warn!("metrics server failed to bind {addr}: {e}; continuing without metrics");
            return Ok(());
        }
    };
    info!("metrics server listening on http://{addr}/metrics");

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                result = listener.accept() => {
                    let Ok((stream, _)) = result else { continue };
                    tokio::spawn(serve_connection(stream, history.clone(), live_state.clone()));
                }
            }
        }
    });

    Ok(())
}

async fn serve_connection(
    mut stream: tokio::net::TcpStream,
    history: Option<Arc<dyn JobHistoryStore>>,
    live_state: Option<Arc<dyn LiveStateProvider>>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let peer = stream
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| UNKNOWN_REQUEST_METADATA.to_owned());
    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let request = String::from_utf8_lossy(&buf[..n]);
    let context = RequestContext {
        peer,
        user_agent: request_header_value(&request, "user-agent")
            .unwrap_or(UNKNOWN_REQUEST_METADATA)
            .to_owned(),
    };
    let first_line = request.lines().next().unwrap_or("");
    let parsed_request = parse_request_line(first_line);

    let response = if parsed_request.as_ref().is_some_and(|request| request.path == "/health") {
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK".to_owned()
    } else if let Err(failure) = crate::auth::authorize(&request) {
        let reason = match failure {
            crate::auth::AuthFailure::Missing => "missing bearer token",
            crate::auth::AuthFailure::Malformed => "malformed authorization header",
            crate::auth::AuthFailure::Mismatch => "invalid bearer token",
        };
        crate::auth::unauthorized_response(reason)
    } else if let Some(parsed_request) = parsed_request {
        if parsed_request.method != "GET" {
            problem_response(
                405,
                "method-not-allowed",
                "Method Not Allowed",
                "Only GET is supported on the coordinator observability HTTP surface.",
            )
        } else {
            route_get(parsed_request, &context, history, live_state).await
        }
    } else {
        problem_response(400, "bad-request", "Bad Request", "Malformed HTTP request line.")
    };

    let _ = stream.write_all(response.as_bytes()).await;
}

async fn route_get(
    request: ParsedRequest<'_>,
    context: &RequestContext,
    history: Option<Arc<dyn JobHistoryStore>>,
    live_state: Option<Arc<dyn LiveStateProvider>>,
) -> String {
    match request.path.as_ref() {
        "/metrics" => metrics_response(),
        "/api/v1/jobs/current" => current_job_response(request.query, history, live_state).await,
        "/api/v1/jobs/recent" => {
            deprecated_endpoint_response(
                "/api/v1/jobs/recent",
                context,
                recent_jobs_response(request.query, history),
            )
            .await
        }
        "/api/v1/jobs/phases/recent" => {
            deprecated_endpoint_response(
                "/api/v1/jobs/phases/recent",
                context,
                recent_job_phases_response(request.query, history),
            )
            .await
        }
        "/api/v1/jobs/stats/recent" => {
            deprecated_endpoint_response(
                "/api/v1/jobs/stats/recent",
                context,
                recent_job_stats_response(request.query, history),
            )
            .await
        }
        "/api/v1/programs/performance" => {
            deprecated_endpoint_response(
                "/api/v1/programs/performance",
                context,
                program_performance_response(request.query, history),
            )
            .await
        }
        "/api/v1/workers" => workers_response(request.query, live_state).await,
        "/api/v1/workers/errors/recent" => {
            deprecated_endpoint_response(
                "/api/v1/workers/errors/recent",
                context,
                recent_worker_errors_response(request.query, history),
            )
            .await
        }
        "/api/v1/coordinators" => {
            deprecated_endpoint_response("/api/v1/coordinators", context, coordinators_response())
                .await
        }
        "/api/v1/openapi.json" => openapi_response(),
        path if path.starts_with("/api/v1/jobs/") => {
            deprecated_endpoint_response(
                "/api/v1/jobs/{job_id}",
                context,
                job_response(path, history),
            )
            .await
        }
        _ => problem_response(404, "not-found", "Not Found", "No route matches this path."),
    }
}

async fn deprecated_endpoint_response<Fut>(
    path: &str,
    context: &RequestContext,
    response: Fut,
) -> String
where
    Fut: Future<Output = String>,
{
    record_deprecated_endpoint_hit(path);
    warn_deprecated_endpoint_once(path, context);
    add_deprecation_headers(response.await)
}

fn record_deprecated_endpoint_hit(path: &str) {
    metrics::counter!("coordinator_deprecated_endpoint_hits_total", "path" => path.to_owned())
        .increment(1);
}

fn warn_deprecated_endpoint_once(path: &str, context: &RequestContext) {
    let now = Utc::now();
    let cutoff = now - Duration::hours(1);
    let key = DeprecatedEndpointWarnKey {
        path: path.to_owned(),
        peer: context.peer.clone(),
        user_agent: context.user_agent.clone(),
    };
    let cache = DEPRECATED_ENDPOINT_WARN_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.retain(|_, seen_at| *seen_at >= cutoff);
    if guard.get(&key).is_some_and(|seen_at| *seen_at >= cutoff) {
        return;
    }
    guard.insert(key, now);
    warn!(
        path,
        peer = %context.peer,
        user_agent = %context.user_agent,
        sunset = DEPRECATED_REST_SUNSET,
        "deprecated coordinator REST endpoint used"
    );
}

fn add_deprecation_headers(response: String) -> String {
    let Some(status_line_end) = response.find("\r\n") else {
        return response;
    };
    let mut with_headers = String::with_capacity(response.len() + DEPRECATED_REST_LINK.len() + 96);
    with_headers.push_str(&response[..status_line_end + 2]);
    with_headers.push_str("Deprecation: true\r\n");
    with_headers.push_str("Sunset: ");
    with_headers.push_str(DEPRECATED_REST_SUNSET);
    with_headers.push_str("\r\nLink: ");
    with_headers.push_str(DEPRECATED_REST_LINK);
    with_headers.push_str("\r\n");
    with_headers.push_str(&response[status_line_end + 2..]);
    with_headers
}

fn request_header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    for line in request.lines().skip(1) {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            break;
        }
        let Some((header_name, value)) = line.split_once(':') else {
            continue;
        };
        if header_name.eq_ignore_ascii_case(name) {
            return Some(value.trim());
        }
    }
    None
}

fn metrics_response() -> String {
    if let Some(handle) = PROMETHEUS_HANDLE.get() {
        let body = handle.render();
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    } else {
        problem_response(
            503,
            "metrics-unavailable",
            "Metrics Unavailable",
            "The Prometheus recorder has not been installed yet.",
        )
    }
}

async fn recent_jobs_response(
    params: HashMap<String, String>,
    history: Option<Arc<dyn JobHistoryStore>>,
) -> String {
    let Some(history) = history else {
        return problem_response(
            503,
            "history-unavailable",
            "History Unavailable",
            "Postgres job history is not enabled for this coordinator.",
        );
    };
    let query = match history_query_from_params(&params) {
        Ok(query) => query,
        Err(response) => return response,
    };
    match history.list_recent_jobs(query).await {
        Ok(mut page) => {
            if let Err(response) = filter_recent_jobs_for_live_view(&mut page.data, &params) {
                return response;
            }
            filter_recent_jobs_by_program(&mut page.data, &params);
            json_response(200, &page)
        }
        Err(error) => problem_response(
            500,
            "history-query-failed",
            "History Query Failed",
            &format!("Failed to read job history: {error:#}"),
        ),
    }
}

async fn current_job_response(
    params: HashMap<String, String>,
    history: Option<Arc<dyn JobHistoryStore>>,
    live_state: Option<Arc<dyn LiveStateProvider>>,
) -> String {
    if let Some(live_state) = live_state {
        let job_id = match parse_optional_job_id(&params) {
            Ok(job_id) => job_id,
            Err(response) => return response,
        };
        let program = params.get("program").map(String::as_str).filter(|value| !value.is_empty());
        return match live_state.current_live_job(job_id, program).await {
            Ok(job) => json_response(
                200,
                &CurrentJobStatusResponse { data: current_job_status_from_live(job) },
            ),
            Err(error) => problem_response(
                500,
                "live-current-job-failed",
                "Live Current Job Failed",
                &format!("Failed to read live current job state: {error:#}"),
            ),
        };
    }

    let Some(history) = history else {
        return problem_response(
            503,
            "history-unavailable",
            "History Unavailable",
            "Postgres job history is not enabled for this coordinator.",
        );
    };

    let mut live_params = params.clone();
    live_params.insert("active".to_owned(), "true".to_owned());
    live_params.entry("limit".to_owned()).or_insert_with(|| "50".to_owned());

    let query = match history_query_from_params(&live_params) {
        Ok(query) => query,
        Err(response) => return response,
    };

    match history.list_recent_jobs(query).await {
        Ok(mut page) => {
            if let Err(response) = filter_recent_jobs_for_live_view(&mut page.data, &live_params) {
                return response;
            }
            filter_recent_jobs_by_program(&mut page.data, &live_params);
            json_response(200, &CurrentJobStatusResponse { data: current_job_status(page.data) })
        }
        Err(error) => problem_response(
            500,
            "history-query-failed",
            "History Query Failed",
            &format!("Failed to read job history: {error:#}"),
        ),
    }
}

fn parse_optional_job_id(
    params: &HashMap<String, String>,
) -> std::result::Result<Option<Uuid>, String> {
    match params.get("job_id").filter(|value| !value.is_empty()) {
        Some(value) => Uuid::parse_str(value).map(Some).map_err(|_| {
            problem_response(400, "invalid-job-id", "Invalid Job ID", "job_id must be a UUID.")
        }),
        None => Ok(None),
    }
}

async fn recent_job_phases_response(
    params: HashMap<String, String>,
    history: Option<Arc<dyn JobHistoryStore>>,
) -> String {
    let Some(history) = history else {
        return problem_response(
            503,
            "history-unavailable",
            "History Unavailable",
            "Postgres job history is not enabled for this coordinator.",
        );
    };
    let active_only = parse_bool_param(&params, "active").unwrap_or(false);
    let pipeline_lane = parse_bool_param(&params, "pipeline").unwrap_or(false);
    let query = match history_query_from_params(&params) {
        Ok(query) => query,
        Err(response) => return response,
    };
    match history.list_recent_jobs(query).await {
        Ok(mut page) => {
            if let Err(response) = filter_recent_jobs_for_live_view(&mut page.data, &params) {
                return response;
            }
            filter_recent_jobs_by_program(&mut page.data, &params);
            json_response(
                200,
                &PhaseLifecyclePage {
                    data: phase_lifecycle_rows(page.data, active_only, pipeline_lane),
                },
            )
        }
        Err(error) => problem_response(
            500,
            "history-query-failed",
            "History Query Failed",
            &format!("Failed to read job history: {error:#}"),
        ),
    }
}

async fn recent_job_stats_response(
    params: HashMap<String, String>,
    history: Option<Arc<dyn JobHistoryStore>>,
) -> String {
    let Some(history) = history else {
        return problem_response(
            503,
            "history-unavailable",
            "History Unavailable",
            "Postgres job history is not enabled for this coordinator.",
        );
    };
    let since_seconds = match parse_since_seconds(&params) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let query = match history_query_from_params(&params) {
        Ok(query) => query,
        Err(response) => return response,
    };
    let sample_limit = query.limit;
    match history.list_recent_jobs(query).await {
        Ok(mut page) => {
            filter_recent_jobs_by_program(&mut page.data, &params);
            json_response(200, &stats_response(page.data, since_seconds, sample_limit))
        }
        Err(error) => problem_response(
            500,
            "history-query-failed",
            "History Query Failed",
            &format!("Failed to read job history: {error:#}"),
        ),
    }
}

async fn program_performance_response(
    params: HashMap<String, String>,
    history: Option<Arc<dyn JobHistoryStore>>,
) -> String {
    let Some(history) = history else {
        return problem_response(
            503,
            "history-unavailable",
            "History Unavailable",
            "Postgres job history is not enabled for this coordinator.",
        );
    };

    let mut query_params = params.clone();
    query_params.entry("limit".to_owned()).or_insert_with(|| "500".to_owned());
    let since_seconds = match parse_since_seconds(&query_params) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let query = match history_query_from_params(&query_params) {
        Ok(query) => query,
        Err(response) => return response,
    };

    match history.list_recent_jobs(query).await {
        Ok(mut page) => {
            filter_recent_jobs_by_program(&mut page.data, &query_params);
            filter_recent_jobs_by_since(&mut page.data, since_seconds);
            json_response(200, &ProgramPerformancePage::from_jobs(page.data))
        }
        Err(error) => problem_response(
            500,
            "history-query-failed",
            "History Query Failed",
            &format!("Failed to read program performance history: {error:#}"),
        ),
    }
}

async fn workers_response(
    params: HashMap<String, String>,
    live_state: Option<Arc<dyn LiveStateProvider>>,
) -> String {
    let Some(live_state) = live_state else {
        return problem_response(
            503,
            "live-state-unavailable",
            "Live State Unavailable",
            "The coordinator live-state provider is not available.",
        );
    };

    match live_state.live_workers().await {
        Ok(mut workers) => {
            let program_filter = program_filter_values(&params);
            if !program_filter.is_empty() {
                workers.retain(|worker| {
                    worker
                        .program
                        .as_deref()
                        .is_some_and(|program| program_filter_matches(&program_filter, program))
                });
            }
            let job_id_filter =
                params.get("job_id").map(String::as_str).filter(|value| !value.is_empty());
            if let Some(job_id) = job_id_filter {
                workers.retain(|worker| worker.job_id.as_deref() == Some(job_id));
            }
            json_response(200, &WorkersPage { data: workers })
        }
        Err(error) => problem_response(
            500,
            "live-workers-failed",
            "Live Workers Failed",
            &format!("Failed to read live worker state: {error:#}"),
        ),
    }
}

async fn coordinators_response() -> String {
    // Single-coordinator deployment: return one entry with locally-known
    // metadata. Discovery of peer coordinators is intentionally a Prometheus
    // concern (target labels), not a runtime concern of this process.
    let coordinator_id = COORDINATOR_ID.get().cloned().unwrap_or_else(|| "unknown".to_owned());
    let version = COORDINATOR_VERSION.get().cloned().unwrap_or_else(|| "unknown".to_owned());
    let environment =
        COORDINATOR_ENVIRONMENT.get().cloned().unwrap_or_else(|| "unknown".to_owned());
    let started_at = COORDINATOR_STARTED_AT.get().copied();
    let row = CoordinatorMetadataRow {
        coordinator_id,
        environment,
        version,
        started_at,
        last_seen_at: Utc::now(),
        // If we are answering this request, the coordinator process is alive
        // by definition; surface the boolean for parity with multi-coord
        // aggregators that compute it from Prometheus `up`.
        up: true,
    };
    json_response(200, &vec![row])
}

async fn recent_worker_errors_response(
    params: HashMap<String, String>,
    history: Option<Arc<dyn JobHistoryStore>>,
) -> String {
    let Some(history) = history else {
        return problem_response(
            503,
            "history-unavailable",
            "History Unavailable",
            "Postgres job history is not enabled for this coordinator.",
        );
    };

    let query = match worker_error_query_from_params(&params) {
        Ok(query) => query,
        Err(response) => return response,
    };

    match history.recent_worker_errors(query).await {
        Ok(events) => {
            let rows: Vec<RecentWorkerErrorRow> =
                events.into_iter().map(RecentWorkerErrorRow::from).collect();
            json_response(200, &WorkersPage { data: rows })
        }
        Err(error) => problem_response(
            500,
            "history-query-failed",
            "History Query Failed",
            &format!("Failed to read worker error history: {error:#}"),
        ),
    }
}

fn worker_error_query_from_params(
    params: &HashMap<String, String>,
) -> Result<WorkerErrorQuery, String> {
    let limit = match params.get("limit").map(|v| v.parse::<usize>()) {
        Some(Ok(value)) => value,
        Some(Err(_)) => {
            return Err(problem_response(
                400,
                "invalid-limit",
                "Invalid limit",
                "limit must be a non-negative integer.",
            ));
        }
        None => 0,
    };
    let since = match params.get("since").map(|v| DateTime::parse_from_rfc3339(v)) {
        Some(Ok(ts)) => Some(ts.with_timezone(&Utc)),
        Some(Err(_)) => {
            return Err(problem_response(
                400,
                "invalid-since",
                "Invalid since",
                "since must be an RFC3339 timestamp (e.g. 2026-05-21T13:00:00Z).",
            ));
        }
        None => None,
    };
    let job_id = match params.get("job_id").filter(|value| !value.is_empty()) {
        Some(value) => match Uuid::parse_str(value) {
            Ok(parsed) => Some(parsed),
            Err(_) => {
                return Err(problem_response(
                    400,
                    "invalid-job-id",
                    "Invalid Job ID",
                    "job_id must be a UUID.",
                ));
            }
        },
        None => None,
    };
    Ok(WorkerErrorQuery {
        limit,
        worker_id: params.get("worker_id").filter(|v| !v.is_empty()).cloned(),
        job_id,
        program: None,
        programs: program_filter_values(params),
        since,
    })
}

impl From<JobHistoryWorkerError> for RecentWorkerErrorRow {
    fn from(event: JobHistoryWorkerError) -> Self {
        Self {
            worker_id: event.worker_id,
            job_id: Some(event.job_id.to_string()),
            program: event.program,
            reason: event.reason,
            message: event.message.unwrap_or_default(),
            occurred_at: event.occurred_at,
        }
    }
}

async fn job_response(path: &str, history: Option<Arc<dyn JobHistoryStore>>) -> String {
    let Some(history) = history else {
        return problem_response(
            503,
            "history-unavailable",
            "History Unavailable",
            "Postgres job history is not enabled for this coordinator.",
        );
    };
    let job_id = path.trim_start_matches("/api/v1/jobs/");
    if job_id.is_empty() || job_id.contains('/') {
        return problem_response(
            404,
            "not-found",
            "Not Found",
            "Use /api/v1/jobs/{job_id} with one UUID job identifier.",
        );
    }
    let Ok(job_id) = Uuid::parse_str(job_id) else {
        return problem_response(400, "invalid-job-id", "Invalid Job ID", "job_id must be a UUID.");
    };
    match history.get_job(job_id).await {
        Ok(Some(job)) => json_response(200, &job),
        Ok(None) => problem_response(
            404,
            "job-not-found",
            "Job Not Found",
            "No job history row exists for the supplied job_id.",
        ),
        Err(error) => problem_response(
            500,
            "history-query-failed",
            "History Query Failed",
            &format!("Failed to read job history: {error:#}"),
        ),
    }
}

fn history_query_from_params(
    params: &HashMap<String, String>,
) -> std::result::Result<JobHistoryListQuery, String> {
    let limit = match params.get("limit") {
        Some(value) => match value.parse::<usize>() {
            Ok(limit @ 1..=500) => limit,
            Ok(_) => {
                return Err(problem_response(
                    400,
                    "invalid-limit",
                    "Invalid Limit",
                    "limit must be an integer between 1 and 500.",
                ));
            }
            Err(_) => {
                return Err(problem_response(
                    400,
                    "invalid-limit",
                    "Invalid Limit",
                    "limit must be an integer between 1 and 500.",
                ));
            }
        },
        None => 50,
    };
    let cursor = match params.get("cursor") {
        Some(value) if !value.is_empty() => match DateTime::parse_from_rfc3339(value) {
            Ok(ts) => Some(ts.with_timezone(&Utc)),
            Err(_) => {
                return Err(problem_response(
                    400,
                    "invalid-cursor",
                    "Invalid Cursor",
                    "cursor must be an RFC3339 timestamp returned by pagination.next_cursor.",
                ));
            }
        },
        _ => None,
    };
    let job_id = match params.get("job_id").filter(|value| !value.is_empty()) {
        Some(value) => match Uuid::parse_str(value) {
            Ok(job_id) => Some(job_id),
            Err(_) => {
                return Err(problem_response(
                    400,
                    "invalid-job-id",
                    "Invalid Job ID",
                    "job_id must be a UUID.",
                ));
            }
        },
        None => None,
    };

    Ok(JobHistoryListQuery {
        coordinator_id: params.get("coordinator_id").cloned().filter(|value| !value.is_empty()),
        job_id,
        state: params.get("state").cloned().filter(|value| !value.is_empty()),
        hash_id: params.get("hash_id").cloned().filter(|value| !value.is_empty()),
        cursor,
        limit,
    })
}

fn parse_bool_param(params: &HashMap<String, String>, name: &str) -> Option<bool> {
    match params.get(name).map(String::as_str) {
        Some("1" | "true" | "yes") => Some(true),
        Some("0" | "false" | "no") => Some(false),
        _ => None,
    }
}

fn parse_since_seconds(
    params: &HashMap<String, String>,
) -> std::result::Result<Option<i64>, String> {
    match params.get("since_seconds") {
        Some(value) if !value.is_empty() => match value.parse::<i64>() {
            Ok(seconds) if seconds > 0 => Ok(Some(seconds)),
            _ => Err(problem_response(
                400,
                "invalid-since-seconds",
                "Invalid Since Seconds",
                "since_seconds must be a positive integer.",
            )),
        },
        _ => Ok(None),
    }
}

fn filter_recent_jobs_by_program(jobs: &mut Vec<JobHistoryJob>, params: &HashMap<String, String>) {
    let allowed = program_filter_values(params);
    if allowed.is_empty() {
        return;
    }

    jobs.retain(|job| program_filter_matches(&allowed, &job.program));
}

fn program_filter_values(params: &HashMap<String, String>) -> Vec<String> {
    let Some(filter) = params.get("program").map(String::as_str).map(str::trim) else {
        return Vec::new();
    };
    if is_all_program_filter(filter) {
        return Vec::new();
    }

    filter
        .trim_start_matches('{')
        .trim_end_matches('}')
        .split(',')
        .map(str::trim)
        .filter(|value| !is_all_program_filter(value))
        .map(ToOwned::to_owned)
        .collect()
}

fn program_filter_matches(allowed: &[String], program: &str) -> bool {
    allowed.is_empty() || allowed.iter().any(|candidate| candidate == program)
}

fn is_all_program_filter(value: &str) -> bool {
    value.is_empty() || matches!(value, "All" | "$__all" | ".*")
}

fn filter_recent_jobs_by_since(jobs: &mut Vec<JobHistoryJob>, since_seconds: Option<i64>) {
    let Some(seconds) = since_seconds else {
        return;
    };
    let since = Utc::now() - Duration::seconds(seconds);
    jobs.retain(|job| job.completed_at.or(job.received_at).unwrap_or(job.sort_at) >= since);
}

fn filter_recent_jobs_for_live_view(
    jobs: &mut Vec<JobHistoryJob>,
    params: &HashMap<String, String>,
) -> std::result::Result<(), String> {
    filter_recent_jobs_for_live_view_since(jobs, params, COORDINATOR_STARTED_AT.get().cloned())
}

fn filter_recent_jobs_for_live_view_since(
    jobs: &mut Vec<JobHistoryJob>,
    params: &HashMap<String, String>,
    process_started_at: Option<DateTime<Utc>>,
) -> std::result::Result<(), String> {
    let active_only = parse_bool_param(params, "active").unwrap_or(false);
    let exclude_stale_active = parse_bool_param(params, "exclude_stale_active").unwrap_or(false);
    let max_update_age_seconds = parse_max_update_age_seconds(params)?;
    if !active_only && !exclude_stale_active && max_update_age_seconds.is_none() {
        return Ok(());
    }
    jobs.retain(|job| {
        let is_active = is_active_state(&job.state);
        (!active_only || is_active)
            && (!(active_only || exclude_stale_active)
                || !is_active
                || process_started_at
                    .map(|started_at| job.updated_at >= started_at)
                    .unwrap_or(true))
            && max_update_age_seconds
                .map(|max_age| job.last_update_age_seconds <= max_age)
                .unwrap_or(true)
    });
    Ok(())
}

fn parse_max_update_age_seconds(
    params: &HashMap<String, String>,
) -> std::result::Result<Option<u64>, String> {
    match params.get("max_update_age_seconds") {
        Some(value) if !value.is_empty() => match value.parse::<u64>() {
            Ok(seconds) if seconds > 0 => Ok(Some(seconds)),
            _ => Err(problem_response(
                400,
                "invalid-max-update-age-seconds",
                "Invalid Max Update Age Seconds",
                "max_update_age_seconds must be a positive integer.",
            )),
        },
        _ => Ok(None),
    }
}

fn phase_lifecycle_rows(
    jobs: Vec<JobHistoryJob>,
    active_only: bool,
    pipeline_lane: bool,
) -> Vec<PhaseLifecycleRow> {
    let mut rows = Vec::new();
    for job in jobs {
        if active_only && !is_active_state(&job.state) {
            continue;
        }
        let job_id = job.job_id.to_string();
        let lane = if pipeline_lane { "Pipeline".to_owned() } else { job.job_label.clone() };
        for timing in &job.phase_timings {
            rows.push(PhaseLifecycleRow {
                lane: lane.clone(),
                coordinator_id: job.coordinator_id.clone(),
                job_id: job_id.clone(),
                job_label: job.job_label.clone(),
                program: job.program.clone(),
                state: job.state.clone(),
                phase: timing.phase.clone(),
                time: timing.start_at,
                end_time: timing.end_at,
                duration_ms: timing.duration_ms,
                workers_count: job.workers_count,
                current: job.current_phase.as_deref() == Some(timing.phase.as_str())
                    && timing.end_at.is_none(),
            });
        }
        if !active_only {
            if let Some((phase, time)) = terminal_marker(&job) {
                rows.push(PhaseLifecycleRow {
                    lane: lane.clone(),
                    coordinator_id: job.coordinator_id.clone(),
                    job_id: job_id.clone(),
                    job_label: job.job_label.clone(),
                    program: job.program.clone(),
                    state: job.state.clone(),
                    phase: if pipeline_lane { "idle" } else { phase }.to_owned(),
                    time,
                    end_time: None,
                    duration_ms: None,
                    workers_count: job.workers_count,
                    current: false,
                });
            }
        }
    }
    rows.sort_by(|left, right| {
        left.lane
            .cmp(&right.lane)
            .then_with(|| left.time.cmp(&right.time))
            .then_with(|| left.job_id.cmp(&right.job_id))
            .then_with(|| left.phase.cmp(&right.phase))
    });
    rows
}

fn current_job_status(jobs: Vec<JobHistoryJob>) -> Vec<CurrentJobStatusRow> {
    let Some(job) = jobs.into_iter().next() else {
        return vec![CurrentJobStatusRow {
            status: "idle".to_owned(),
            phase: "idle".to_owned(),
            phase_code: phase_code("idle"),
            coordinator_id: None,
            job_id: None,
            state: None,
            age_seconds: None,
            phase_age_seconds: None,
            update_age_seconds: None,
            workers_count: None,
        }];
    };

    let phase = job
        .current_phase
        .clone()
        .unwrap_or_else(|| if job.state == "Created" { "queued" } else { "unknown" }.to_owned());

    vec![CurrentJobStatusRow {
        status: "running".to_owned(),
        phase_code: phase_code(&phase),
        phase,
        coordinator_id: Some(job.coordinator_id),
        job_id: Some(job.job_id.to_string()),
        state: Some(job.state),
        age_seconds: job.age_seconds,
        phase_age_seconds: job.current_phase_age_seconds,
        update_age_seconds: Some(job.last_update_age_seconds),
        workers_count: Some(job.workers_count),
    }]
}

fn current_job_status_from_live(job: Option<LiveJobSnapshot>) -> Vec<CurrentJobStatusRow> {
    let Some(job) = job else {
        return vec![CurrentJobStatusRow {
            status: "idle".to_owned(),
            phase: "idle".to_owned(),
            phase_code: phase_code("idle"),
            coordinator_id: None,
            job_id: None,
            state: None,
            age_seconds: None,
            phase_age_seconds: None,
            update_age_seconds: None,
            workers_count: None,
        }];
    };

    vec![CurrentJobStatusRow {
        status: "running".to_owned(),
        phase_code: phase_code(&job.phase),
        phase: job.phase,
        coordinator_id: Some(job.coordinator_id),
        job_id: Some(job.job_id.to_string()),
        state: Some(job.state),
        age_seconds: job.age_seconds,
        phase_age_seconds: job.phase_age_seconds,
        update_age_seconds: Some(job.update_age_seconds),
        workers_count: Some(job.workers_count),
    }]
}

fn phase_code(phase: &str) -> u8 {
    match phase {
        "idle" => 0,
        "queued" => 1,
        "Contributions" => 2,
        "Prove" => 3,
        "Aggregate" => 4,
        "Execution" => 5,
        _ => 255,
    }
}

fn terminal_marker(job: &JobHistoryJob) -> Option<(&'static str, DateTime<Utc>)> {
    let phase = outcome_for_state(&job.state)?;
    let time = job
        .completed_at
        .or_else(|| job.phase_timings.iter().filter_map(|timing| timing.end_at).max())
        .unwrap_or(job.updated_at);
    Some((phase, time))
}

fn stats_response(
    jobs: Vec<JobHistoryJob>,
    since_seconds: Option<i64>,
    sample_limit: usize,
) -> JobHistoryStatsResponse {
    let since = since_seconds.map(|seconds| Utc::now() - Duration::seconds(seconds));
    let mut by_outcome: HashMap<String, Vec<u64>> = HashMap::new();

    for job in jobs {
        let Some(outcome) = outcome_for_state(&job.state) else {
            continue;
        };
        let terminal_at = job.completed_at.unwrap_or(job.sort_at);
        if since.is_some_and(|since| terminal_at < since) {
            continue;
        }
        if let Some(duration_ms) = job.duration_ms {
            by_outcome.entry(outcome.to_owned()).or_default().push(duration_ms);
        } else {
            by_outcome.entry(outcome.to_owned()).or_default();
        }
    }

    let success_count = by_outcome.get("success").map_or(0, Vec::len);
    let failure_count = by_outcome.get("failure").map_or(0, Vec::len);
    let cancelled_count = by_outcome.get("cancelled").map_or(0, Vec::len);
    let all_durations = by_outcome.values().flatten().copied().collect::<Vec<_>>();

    let mut outcomes = by_outcome
        .into_iter()
        .map(|(outcome, durations)| outcome_stats(outcome, durations))
        .collect::<Vec<_>>();
    outcomes.sort_by(|left, right| {
        outcome_sort_key(&left.outcome).cmp(&outcome_sort_key(&right.outcome))
    });

    JobHistoryStatsResponse {
        data: vec![JobHistoryStatsSummary {
            window_seconds: since_seconds,
            sample_limit,
            terminal_jobs: success_count + failure_count + cancelled_count,
            success_count,
            failure_count,
            cancelled_count,
            avg_duration_ms: average(&all_durations),
            p50_duration_ms: quantile(&mut all_durations.clone(), 0.50),
            p95_duration_ms: quantile(&mut all_durations.clone(), 0.95),
            p99_duration_ms: quantile(&mut all_durations.clone(), 0.99),
            max_duration_ms: all_durations.iter().copied().max(),
        }],
        outcomes,
    }
}

fn outcome_stats(outcome: String, durations: Vec<u64>) -> JobHistoryOutcomeStats {
    JobHistoryOutcomeStats {
        outcome,
        jobs: durations.len(),
        avg_duration_ms: average(&durations),
        p50_duration_ms: quantile(&mut durations.clone(), 0.50),
        p95_duration_ms: quantile(&mut durations.clone(), 0.95),
        p99_duration_ms: quantile(&mut durations.clone(), 0.99),
        max_duration_ms: durations.iter().copied().max(),
    }
}

fn average(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    Some((values.iter().sum::<u64>() as f64 / values.len() as f64).round() as u64)
}

fn quantile(values: &mut [u64], quantile: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let rank = ((values.len() as f64 * quantile).ceil() as usize).saturating_sub(1);
    values.get(rank.min(values.len() - 1)).copied()
}

fn outcome_for_state(state: &str) -> Option<&'static str> {
    match state {
        "Completed" => Some("success"),
        "Failed" => Some("failure"),
        "Cancelled" => Some("cancelled"),
        _ => None,
    }
}

fn outcome_sort_key(outcome: &str) -> u8 {
    match outcome {
        "success" => 0,
        "failure" => 1,
        "cancelled" => 2,
        _ => 3,
    }
}

fn is_active_state(state: &str) -> bool {
    !matches!(state, "Completed" | "Failed" | "Cancelled")
}

#[derive(Debug)]
struct ParsedRequest<'a> {
    method: &'a str,
    path: Cow<'a, str>,
    query: HashMap<String, String>,
}

fn parse_request_line(line: &str) -> Option<ParsedRequest<'_>> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    let _version = parts.next()?;
    let (path, raw_query) = target.split_once('?').unwrap_or((target, ""));
    Some(ParsedRequest { method, path: percent_decode(path).ok()?, query: parse_query(raw_query) })
}

fn parse_query(raw_query: &str) -> HashMap<String, String> {
    raw_query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some((percent_decode(key).ok()?.into_owned(), percent_decode(value).ok()?.into_owned()))
        })
        .collect()
}

fn percent_decode(value: &str) -> std::result::Result<Cow<'_, str>, ()> {
    if !value.as_bytes().iter().any(|byte| matches!(byte, b'%' | b'+')) {
        return Ok(Cow::Borrowed(value));
    }
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                decoded.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = from_hex(bytes[i + 1]).ok_or(())?;
                let lo = from_hex(bytes[i + 2]).ok_or(())?;
                decoded.push((hi << 4) | lo);
                i += 3;
            }
            b'%' => return Err(()),
            byte => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(decoded).map(Cow::Owned).map_err(|_| ())
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn json_response<T: serde::Serialize>(status: u16, value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(body) => http_response(status, "application/json", &body),
        Err(error) => problem_response(
            500,
            "json-serialization-failed",
            "JSON Serialization Failed",
            &format!("Failed to serialize response body: {error}"),
        ),
    }
}

fn problem_response(status: u16, slug: &str, title: &str, detail: &str) -> String {
    let body = serde_json::json!({
        "type": format!("https://zisk.dev/problems/{slug}"),
        "title": title,
        "status": status,
        "detail": detail,
    });
    http_response(status, "application/problem+json", &body.to_string())
}

fn http_response(status: u16, content_type: &str, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn openapi_response() -> String {
    json_response(
        200,
        &serde_json::json!({
            "openapi": "3.1.0",
            "info": {
                "title": "ZisK Coordinator Observability API",
                "version": "1.0.0"
            },
            "paths": {
                "/api/v1/jobs/current": {
                    "get": {
                        "summary": "Get current proof status for operator hero cards",
                        "parameters": [
                            {"name": "coordinator_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "job_id", "in": "query", "schema": {"type": "string", "format": "uuid"}},
                            {"name": "hash_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "program", "in": "query", "schema": {"type": "string"}},
                            {"name": "max_update_age_seconds", "in": "query", "schema": {"type": "integer"}},
                            {"name": "limit", "in": "query", "schema": {"type": "integer", "default": 50, "maximum": 500}}
                        ],
                        "responses": {
                            "200": {
                                "description": "One current status row; idle is represented explicitly",
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/CurrentJobStatusResponse"}
                                    }
                                }
                            }
                        }
                    }
                },
                "/api/v1/jobs/recent": {
                    "get": {
                        "deprecated": true,
                        "summary": "List recent job history rows",
                        "parameters": [
                            {"name": "coordinator_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "job_id", "in": "query", "schema": {"type": "string", "format": "uuid"}},
                            {"name": "state", "in": "query", "schema": {"type": "string"}},
                            {"name": "hash_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "program", "in": "query", "schema": {"type": "string"}},
                            {"name": "active", "in": "query", "schema": {"type": "boolean", "default": false}},
                            {"name": "exclude_stale_active", "in": "query", "schema": {"type": "boolean", "default": false}},
                            {"name": "max_update_age_seconds", "in": "query", "schema": {"type": "integer"}},
                            {"name": "cursor", "in": "query", "schema": {"type": "string", "format": "date-time"}},
                            {"name": "limit", "in": "query", "schema": {"type": "integer", "default": 50, "maximum": 500}}
                        ],
                        "responses": {
                            "200": {
                                "description": "Recent job history page",
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/JobHistoryPage"}
                                    }
                                }
                            }
                        }
                    }
                },
                "/api/v1/jobs/phases/recent": {
                    "get": {
                        "deprecated": true,
                        "summary": "List flattened recent job phase lifecycle rows",
                        "parameters": [
                            {"name": "coordinator_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "job_id", "in": "query", "schema": {"type": "string", "format": "uuid"}},
                            {"name": "hash_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "program", "in": "query", "schema": {"type": "string"}},
                            {"name": "active", "in": "query", "schema": {"type": "boolean", "default": false}},
                            {"name": "exclude_stale_active", "in": "query", "schema": {"type": "boolean", "default": false}},
                            {"name": "pipeline", "in": "query", "schema": {"type": "boolean", "default": false}},
                            {"name": "max_update_age_seconds", "in": "query", "schema": {"type": "integer"}},
                            {"name": "limit", "in": "query", "schema": {"type": "integer", "default": 50, "maximum": 500}}
                        ],
                        "responses": {
                            "200": {
                                "description": "Flattened phase lifecycle rows",
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/PhaseLifecyclePage"}
                                    }
                                }
                            }
                        }
                    }
                },
                "/api/v1/jobs/stats/recent": {
                    "get": {
                        "deprecated": true,
                        "summary": "Compute recent history-backed job outcome and duration stats",
                        "parameters": [
                            {"name": "coordinator_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "job_id", "in": "query", "schema": {"type": "string", "format": "uuid"}},
                            {"name": "hash_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "program", "in": "query", "schema": {"type": "string"}},
                            {"name": "since_seconds", "in": "query", "schema": {"type": "integer", "default": 86400}},
                            {"name": "limit", "in": "query", "schema": {"type": "integer", "default": 500, "maximum": 500}}
                        ],
                        "responses": {
                            "200": {
                                "description": "Recent outcome and duration stats",
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/JobHistoryStatsResponse"}
                                    }
                                }
                            }
                        }
                    }
                },
                "/api/v1/jobs/{job_id}": {
                    "get": {
                        "deprecated": true,
                        "summary": "Get one job history row",
                        "parameters": [
                            {"name": "job_id", "in": "path", "required": true, "schema": {"type": "string", "format": "uuid"}}
                        ],
                        "responses": {
                            "200": {
                                "description": "One job history row",
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/JobHistoryJob"}
                                    }
                                }
                            }
                        }
                    }
                },
                "/api/v1/coordinators": {
                    "get": {
                        "deprecated": true,
                        "summary": "List coordinator processes known to this metrics endpoint",
                        "responses": {
                            "200": {
                                "description": "Coordinator metadata rows (one entry for single-coordinator deployments)",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "array",
                                            "items": {"$ref": "#/components/schemas/CoordinatorMetadataRow"}
                                        }
                                    }
                                }
                            }
                        }
                        }
                    },
                    "/api/v1/workers": {
                        "get": {
                            "summary": "List live workers known to the coordinator process",
                            "parameters": [
                                {"name": "program", "in": "query", "schema": {"type": "string"}},
                                {"name": "job_id", "in": "query", "schema": {"type": "string", "format": "uuid"}}
                            ],
                            "responses": {
                                "200": {
                                    "description": "Live worker rows in a data envelope",
                                    "content": {
                                        "application/json": {
                                            "schema": {
                                                "type": "object",
                                                "required": ["data"],
                                                "properties": {
                                                    "data": {
                                                        "type": "array",
                                                        "items": {"$ref": "#/components/schemas/WorkerSnapshot"}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "/api/v1/workers/errors/recent": {
                    "get": {
                        "deprecated": true,
                        "summary": "List recent worker-scoped error events",
                        "description": "Returns a data envelope backed by `job_history_worker_errors`. `program=All`, `$__all`, and `.*` mean no program filter; comma/list values are treated as multiple aliases.",
                        "parameters": [
                                {"name": "limit", "in": "query", "schema": {"type": "integer", "default": 50, "maximum": 500}},
                                {"name": "worker_id", "in": "query", "schema": {"type": "string"}},
                                {"name": "job_id", "in": "query", "schema": {"type": "string", "format": "uuid"}},
                                {"name": "program", "in": "query", "schema": {"type": "string"}},
                            {"name": "since", "in": "query", "schema": {"type": "string", "format": "date-time"}}
                        ],
                        "responses": {
                            "200": {
                                "description": "Recent worker error events",
                                "content": {
                                        "application/json": {
                                            "schema": {
                                                "type": "object",
                                                "required": ["data"],
                                                "properties": {
                                                    "data": {
                                                        "type": "array",
                                                        "items": {"$ref": "#/components/schemas/RecentWorkerErrorRow"}
                                                    }
                                                }
                                            }
                                        }
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "JobHistoryPage": {
                        "type": "object",
                        "required": ["data", "pagination"],
                        "properties": {
                            "data": {
                                "type": "array",
                                "items": {"$ref": "#/components/schemas/JobHistoryJob"}
                            },
                            "pagination": {"type": "object"}
                        }
                    },
                    "JobHistoryJob": {
                        "type": "object",
                        "required": [
                            "coordinator_id",
                            "job_id",
                            "job_label",
                            "hash_id",
                            "program",
                            "state",
                            "proof_type",
                            "workers",
                            "workers_count",
                            "phase_timings",
                            "last_update_age_seconds",
                            "updated_at",
                            "sort_at"
                        ],
                        "properties": {
                            "coordinator_id": {"type": "string"},
                            "job_id": {"type": "string", "format": "uuid"},
                            "job_label": {"type": "string"},
                            "hash_id": {"type": "string"},
                            "program": {"type": "string"},
                            "state": {"type": "string"},
                            "failure_reason": {"type": ["string", "null"]},
                            "proof_type": {"type": "string"},
                            "received_at": {"type": ["string", "null"], "format": "date-time"},
                            "completed_at": {"type": ["string", "null"], "format": "date-time"},
                            "duration_ms": {"type": ["integer", "null"]},
                            "workers": {"type": "array", "items": {"type": "string"}},
                            "workers_count": {"type": "integer"},
                            "agg_worker_id": {"type": ["string", "null"]},
                            "phase_timings": {
                                "type": "array",
                                "items": {"$ref": "#/components/schemas/PhaseTiming"}
                            },
                            "contributions_duration_ms": {"type": ["integer", "null"]},
                            "prove_duration_ms": {"type": ["integer", "null"]},
                            "aggregate_duration_ms": {"type": ["integer", "null"]},
                            "execution_duration_ms": {"type": ["integer", "null"]},
                            "age_seconds": {"type": ["integer", "null"]},
                            "current_phase": {"type": ["string", "null"]},
                            "current_phase_started_at": {
                                "type": ["string", "null"],
                                "format": "date-time"
                            },
                            "current_phase_age_seconds": {"type": ["integer", "null"]},
                            "last_update_age_seconds": {"type": "integer"},
                            "instances": {"type": ["integer", "null"]},
                            "executed_steps": {"type": ["integer", "null"]},
                            "updated_at": {"type": "string", "format": "date-time"},
                            "sort_at": {"type": "string", "format": "date-time"}
                        }
                    },
                    "PhaseTiming": {
                        "type": "object",
                        "required": ["phase", "start_at"],
                        "properties": {
                            "phase": {"type": "string"},
                            "start_at": {"type": "string", "format": "date-time"},
                            "end_at": {"type": ["string", "null"], "format": "date-time"},
                            "duration_ms": {"type": ["integer", "null"]}
                        }
                    },
                    "PhaseLifecyclePage": {
                        "type": "object",
                        "required": ["data"],
                        "properties": {
                            "data": {
                                "type": "array",
                                "items": {"$ref": "#/components/schemas/PhaseLifecycleRow"}
                            }
                        }
                    },
                    "PhaseLifecycleRow": {
                        "type": "object",
                        "required": ["lane", "coordinator_id", "job_id", "job_label", "program", "state", "phase", "time", "workers_count", "current"],
                        "properties": {
                            "lane": {"type": "string"},
                            "coordinator_id": {"type": "string"},
                            "job_id": {"type": "string", "format": "uuid"},
                            "job_label": {"type": "string"},
                            "program": {"type": "string"},
                            "state": {"type": "string"},
                            "phase": {"type": "string"},
                            "time": {"type": "string", "format": "date-time"},
                            "end_time": {"type": ["string", "null"], "format": "date-time"},
                            "duration_ms": {"type": ["integer", "null"]},
                            "workers_count": {"type": "integer"},
                            "current": {"type": "boolean"}
                        }
                    },
                    "CurrentJobStatusResponse": {
                        "type": "object",
                        "required": ["data"],
                        "properties": {
                            "data": {
                                "type": "array",
                                "items": {"$ref": "#/components/schemas/CurrentJobStatusRow"},
                                "minItems": 1,
                                "maxItems": 1
                            }
                        }
                    },
                    "CurrentJobStatusRow": {
                        "type": "object",
                        "required": ["status", "phase", "phase_code"],
                        "properties": {
                            "status": {"type": "string", "enum": ["idle", "running"]},
                            "phase": {"type": "string"},
                            "phase_code": {
                                "type": "integer",
                                "description": "Stable numeric phase code for Grafana stat value mappings: idle=0, queued=1, Contributions=2, Prove=3, Aggregate=4, Execution=5, unknown=255"
                            },
                            "coordinator_id": {"type": ["string", "null"]},
                            "job_id": {"type": ["string", "null"], "format": "uuid"},
                            "state": {"type": ["string", "null"]},
                            "age_seconds": {"type": ["integer", "null"]},
                            "phase_age_seconds": {"type": ["integer", "null"]},
                            "update_age_seconds": {"type": ["integer", "null"]},
                            "workers_count": {"type": ["integer", "null"]}
                        }
                    },
                    "JobHistoryStatsResponse": {
                        "type": "object",
                        "required": ["data", "outcomes"],
                        "properties": {
                            "data": {
                                "type": "array",
                                "items": {"$ref": "#/components/schemas/JobHistoryStatsSummary"}
                            },
                            "outcomes": {
                                "type": "array",
                                "items": {"$ref": "#/components/schemas/JobHistoryOutcomeStats"}
                            }
                        }
                    },
                    "JobHistoryStatsSummary": {
                        "type": "object",
                        "properties": {
                            "window_seconds": {"type": ["integer", "null"]},
                            "sample_limit": {"type": "integer"},
                            "terminal_jobs": {"type": "integer"},
                            "success_count": {"type": "integer"},
                            "failure_count": {"type": "integer"},
                            "cancelled_count": {"type": "integer"},
                            "avg_duration_ms": {"type": ["integer", "null"]},
                            "p50_duration_ms": {"type": ["integer", "null"]},
                            "p95_duration_ms": {"type": ["integer", "null"]},
                            "p99_duration_ms": {"type": ["integer", "null"]},
                            "max_duration_ms": {"type": ["integer", "null"]}
                        }
                    },
                    "JobHistoryOutcomeStats": {
                        "type": "object",
                        "properties": {
                            "outcome": {"type": "string"},
                            "jobs": {"type": "integer"},
                            "avg_duration_ms": {"type": ["integer", "null"]},
                            "p50_duration_ms": {"type": ["integer", "null"]},
                            "p95_duration_ms": {"type": ["integer", "null"]},
                            "p99_duration_ms": {"type": ["integer", "null"]},
                            "max_duration_ms": {"type": ["integer", "null"]}
                        }
                    },
                        "CoordinatorMetadataRow": {
                            "type": "object",
                            "required": ["coordinator_id", "environment", "version", "last_seen_at", "up"],
                            "properties": {
                                "coordinator_id": {"type": "string"},
                            "environment": {"type": "string"},
                            "version": {"type": "string"},
                            "started_at": {"type": ["string", "null"], "format": "date-time"},
                            "last_seen_at": {"type": "string", "format": "date-time"},
                                "up": {"type": "boolean"}
                            }
                        },
                        "WorkerSnapshot": {
                            "type": "object",
                            "required": ["worker_id", "status", "heartbeat_age_seconds", "compute_units", "updated_at"],
                            "properties": {
                                "worker_id": {"type": "string"},
                                "status": {"type": "string"},
                                "program": {"type": ["string", "null"]},
                                "job_id": {"type": ["string", "null"]},
                                "job_label": {"type": ["string", "null"]},
                                "phase": {"type": ["string", "null"]},
                                "heartbeat_age_seconds": {"type": "integer"},
                                "assigned_seconds": {"type": ["integer", "null"]},
                                "compute_units": {"type": "integer"},
                                "updated_at": {"type": "string", "format": "date-time"}
                            }
                        },
                        "RecentWorkerErrorRow": {
                        "type": "object",
                        "required": ["worker_id", "program", "reason", "message", "occurred_at"],
                        "properties": {
                            "worker_id": {"type": "string"},
                            "job_id": {"type": ["string", "null"]},
                            "program": {"type": "string"},
                            "reason": {
                                "type": "string",
                                "description": "Bounded taxonomy: heartbeat_lost, channel_closed, setup_fail, prove_fail, agg_fail, unreachable, unknown"
                            },
                            "message": {"type": "string"},
                            "occurred_at": {"type": "string", "format": "date-time"}
                        }
                    }
                }
            }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_history_query() {
        let request =
            parse_request_line("GET /api/v1/jobs/recent?limit=25&coordinator_id=coord-a HTTP/1.1")
                .unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/v1/jobs/recent");
        assert_eq!(request.query.get("limit").unwrap(), "25");
        assert_eq!(request.query.get("coordinator_id").unwrap(), "coord-a");
    }

    #[test]
    fn decodes_query_values() {
        let request =
            parse_request_line("GET /api/v1/jobs/recent?cursor=2026-05-18T01%3A02%3A03Z HTTP/1.1")
                .unwrap();
        assert_eq!(request.query.get("cursor").unwrap(), "2026-05-18T01:02:03Z");
    }

    #[test]
    fn extracts_request_header_values_case_insensitively() {
        let request = "GET /metrics HTTP/1.1\r\nHost: coord\r\nUser-Agent: grafana\r\n\r\n";

        assert_eq!(request_header_value(request, "user-agent"), Some("grafana"));
    }

    #[test]
    fn adds_deprecation_headers_after_status_line() {
        let response = json_response(200, &serde_json::json!({"ok": true}));
        let response = add_deprecation_headers(response);

        assert!(response.starts_with("HTTP/1.1 200 OK\r\nDeprecation: true\r\n"));
        assert!(response.contains("Sunset: Wed, 30 Sep 2026 00:00:00 GMT\r\n"));
        assert!(response.contains("Link: </api/v1/openapi.json>; rel=\"deprecation\""));
        assert!(response.contains("\r\nContent-Type: application/json\r\n"));
    }

    #[test]
    fn rejects_invalid_history_limit() {
        let mut params = HashMap::new();
        params.insert("limit".to_owned(), "abc".to_owned());
        let response = history_query_from_params(&params).unwrap_err();
        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("invalid-limit"));
    }

    #[test]
    fn parses_optional_job_id_filter() {
        let job_id = Uuid::new_v4();
        let params = HashMap::from([("job_id".to_owned(), job_id.to_string())]);

        let query = history_query_from_params(&params).unwrap();

        assert_eq!(query.job_id, Some(job_id));
    }

    #[test]
    fn rejects_invalid_job_id_filter() {
        let params = HashMap::from([("job_id".to_owned(), "not-a-uuid".to_owned())]);

        let response = history_query_from_params(&params).unwrap_err();

        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("invalid-job-id"));
    }

    #[test]
    fn program_filter_values_normalize_grafana_all_and_multi_select() {
        for all_value in ["", "All", "$__all", ".*"] {
            let params = HashMap::from([("program".to_owned(), all_value.to_owned())]);
            assert!(program_filter_values(&params).is_empty());
        }

        let params = HashMap::from([("program".to_owned(), "{alpha,beta}".to_owned())]);
        assert_eq!(program_filter_values(&params), vec!["alpha", "beta"]);
        assert!(program_filter_matches(&program_filter_values(&params), "alpha"));
        assert!(!program_filter_matches(&program_filter_values(&params), "gamma"));
    }

    #[test]
    fn worker_error_query_normalizes_program_and_job_filters() {
        let job_id = Uuid::new_v4();
        let params = HashMap::from([
            ("limit".to_owned(), "50".to_owned()),
            ("worker_id".to_owned(), "worker-a".to_owned()),
            ("job_id".to_owned(), job_id.to_string()),
            ("program".to_owned(), "{alpha,beta}".to_owned()),
        ]);

        let query = worker_error_query_from_params(&params).unwrap();

        assert_eq!(query.limit, 50);
        assert_eq!(query.worker_id.as_deref(), Some("worker-a"));
        assert_eq!(query.job_id, Some(job_id));
        assert!(query.program.is_none());
        assert_eq!(query.programs, vec!["alpha", "beta"]);
    }

    fn history_job(
        state: &str,
        duration_ms: Option<u64>,
        phase_timings: Vec<zisk_coordinator::JobHistoryPhaseTiming>,
    ) -> JobHistoryJob {
        let now = Utc::now();
        JobHistoryJob {
            coordinator_id: "coord-a".to_owned(),
            job_id: Uuid::new_v4(),
            job_label: "test-job".to_owned(),
            hash_id: "hash-a".to_owned(),
            program: "hash-a".to_owned(),
            state: state.to_owned(),
            failure_reason: None,
            proof_type: "VadcopFinal".to_owned(),
            received_at: Some(now - Duration::seconds(20)),
            completed_at: if outcome_for_state(state).is_some() { Some(now) } else { None },
            duration_ms,
            workers: vec!["worker-a".to_owned()],
            workers_count: 1,
            agg_worker_id: None,
            phase_timings,
            contributions_duration_ms: None,
            prove_duration_ms: None,
            aggregate_duration_ms: None,
            execution_duration_ms: None,
            age_seconds: Some(20),
            current_phase: if state.starts_with("Running") {
                Some("Prove".to_owned())
            } else {
                None
            },
            current_phase_started_at: Some(now - Duration::seconds(10)),
            current_phase_age_seconds: Some(10),
            last_update_age_seconds: 5,
            instances: None,
            executed_steps: None,
            updated_at: now,
            sort_at: now,
        }
    }

    #[test]
    fn phase_lifecycle_rows_flatten_named_phase_windows() {
        let now = Utc::now();
        let job = history_job(
            "Running (Prove)",
            None,
            vec![
                zisk_coordinator::JobHistoryPhaseTiming {
                    phase: "Contributions".to_owned(),
                    start_at: now - Duration::seconds(20),
                    end_at: Some(now - Duration::seconds(10)),
                    duration_ms: Some(10_000),
                },
                zisk_coordinator::JobHistoryPhaseTiming {
                    phase: "Prove".to_owned(),
                    start_at: now - Duration::seconds(10),
                    end_at: None,
                    duration_ms: None,
                },
            ],
        );

        let job_label = job.job_label.clone();
        let rows = phase_lifecycle_rows(vec![job], true, false);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].lane, job_label);
        assert_eq!(rows[0].job_label, job_label);
        assert_eq!(rows[0].phase, "Contributions");
        assert_eq!(rows[1].phase, "Prove");
        assert!(rows[1].current);
    }

    #[test]
    fn pipeline_lifecycle_uses_single_lane_and_idle_after_terminal_job() {
        let now = Utc::now();
        let job = history_job(
            "Completed",
            Some(20_000),
            vec![zisk_coordinator::JobHistoryPhaseTiming {
                phase: "Prove".to_owned(),
                start_at: now - Duration::seconds(20),
                end_at: Some(now),
                duration_ms: Some(20_000),
            }],
        );

        let rows = phase_lifecycle_rows(vec![job], false, true);

        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.lane == "Pipeline"));
        assert!(rows.iter().all(|row| row.job_label == "test-job"));
        assert_eq!(rows[0].phase, "Prove");
        assert_eq!(rows[1].phase, "idle");
    }

    #[test]
    fn current_job_status_returns_explicit_idle_row_when_empty() {
        let rows = current_job_status(Vec::new());

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, "idle");
        assert_eq!(rows[0].phase, "idle");
        assert_eq!(rows[0].phase_code, 0);
        assert!(rows[0].job_id.is_none());
        assert!(rows[0].age_seconds.is_none());
    }

    #[test]
    fn current_job_status_uses_newest_live_job_phase_and_age() {
        let mut job = history_job("Running (Contributions)", None, Vec::new());
        job.current_phase = Some("Contributions".to_owned());
        job.age_seconds = Some(42);
        job.current_phase_age_seconds = Some(12);

        let rows = current_job_status(vec![job]);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, "running");
        assert_eq!(rows[0].phase, "Contributions");
        assert_eq!(rows[0].phase_code, 2);
        assert_eq!(rows[0].age_seconds, Some(42));
        assert_eq!(rows[0].phase_age_seconds, Some(12));
        assert_eq!(rows[0].workers_count, Some(1));
    }

    #[test]
    fn stats_response_counts_terminal_outcomes_and_quantiles() {
        let jobs = vec![
            history_job("Completed", Some(100), Vec::new()),
            history_job("Failed", Some(300), Vec::new()),
            history_job("Failed", Some(500), Vec::new()),
            history_job("Running (Prove)", None, Vec::new()),
        ];

        let stats = stats_response(jobs, Some(86_400), 500);
        let summary = &stats.data[0];
        assert_eq!(summary.success_count, 1);
        assert_eq!(summary.failure_count, 2);
        assert_eq!(summary.terminal_jobs, 3);
        assert_eq!(summary.p95_duration_ms, Some(500));
        assert_eq!(stats.outcomes.iter().find(|row| row.outcome == "failure").unwrap().jobs, 2);
    }

    #[test]
    fn live_view_filter_drops_stale_running_history_rows() {
        let mut fresh = history_job("Running (Prove)", None, Vec::new());
        fresh.last_update_age_seconds = 30;
        let mut stale = history_job("Running (Prove)", None, Vec::new());
        stale.last_update_age_seconds = 600;
        let mut terminal = history_job("Completed", Some(100), Vec::new());
        terminal.last_update_age_seconds = 10;
        let mut jobs = vec![fresh, stale, terminal];
        let params = HashMap::from([
            ("active".to_owned(), "true".to_owned()),
            ("max_update_age_seconds".to_owned(), "300".to_owned()),
        ]);

        filter_recent_jobs_for_live_view(&mut jobs, &params).unwrap();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].last_update_age_seconds, 30);
    }

    #[test]
    fn live_view_filter_drops_running_rows_from_previous_process() {
        let process_started_at = Utc::now();
        let mut current_process = history_job("Running (Prove)", None, Vec::new());
        current_process.updated_at = process_started_at + Duration::seconds(5);
        current_process.last_update_age_seconds = 5;
        let mut previous_process = history_job("Running (Prove)", None, Vec::new());
        previous_process.updated_at = process_started_at - Duration::seconds(60);
        previous_process.last_update_age_seconds = 60;
        let mut jobs = vec![previous_process, current_process];
        let params = HashMap::from([
            ("active".to_owned(), "true".to_owned()),
            ("max_update_age_seconds".to_owned(), "300".to_owned()),
        ]);

        filter_recent_jobs_for_live_view_since(&mut jobs, &params, Some(process_started_at))
            .unwrap();

        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].updated_at >= process_started_at);
    }

    #[test]
    fn live_view_filter_can_keep_terminal_history_while_dropping_stale_active_rows() {
        let process_started_at = Utc::now();
        let mut completed = history_job("Completed", Some(100), Vec::new());
        completed.updated_at = process_started_at - Duration::minutes(30);
        let mut stale_active = history_job("Running (Prove)", None, Vec::new());
        stale_active.updated_at = process_started_at - Duration::minutes(10);
        let mut current_active = history_job("Running (Prove)", None, Vec::new());
        current_active.updated_at = process_started_at + Duration::seconds(5);
        let mut jobs = vec![completed, stale_active, current_active];
        let params = HashMap::from([("exclude_stale_active".to_owned(), "true".to_owned())]);

        filter_recent_jobs_for_live_view_since(&mut jobs, &params, Some(process_started_at))
            .unwrap();

        assert_eq!(jobs.len(), 2);
        assert!(jobs.iter().any(|job| job.state == "Completed"));
        assert!(jobs
            .iter()
            .any(|job| { job.state == "Running (Prove)" && job.updated_at >= process_started_at }));
    }

    #[test]
    fn worker_metric_seed_covers_all_pool_states() {
        assert_eq!(
            WORKER_STATUS_LABELS,
            ["ready", "idle", "setting_up", "running", "disconnected", "connecting", "error",]
        );
    }
}
