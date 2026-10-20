//! Prometheus metrics registration and HTTP scrape endpoint.
//!
//! ## Exposed metrics
//!
//! Recorded as things happen:
//!
//! | Name | Type | Labels | Description |
//! |------|------|--------|-------------|
//! | `coordinator_info` | Gauge | `name`, `environment`, `version` | Always 1: who this coordinator is |
//! | `coordinator_requests_total` | Counter | `method`, `status` | gRPC calls by method and outcome |
//! | `coordinator_request_duration_seconds` | Histogram | `method` | gRPC call latency |
//! | `coordinator_api_connections` | Gauge | `ip` | Open connections to the API, by client address |
//! | `coordinator_active_jobs` | Gauge | — | Currently active (non-terminal) jobs |
//! | `coordinator_jobs_total` | Counter | `kind`, `outcome`, `reason` | Jobs by kind and final outcome; failures by reason class |
//! | `coordinator_worker_jobs_total` | Counter | `worker_id`, `outcome` | Per-worker participation count by job outcome |
//! | `coordinator_job_duration_seconds` | Histogram | `outcome` | End-to-end job duration (Contributions start → terminal state) |
//! | `coordinator_job_steps_total` | Counter | `kind` | Steps of the jobs that succeeded |
//! | `coordinator_proof_phase_seconds` | Histogram | `phase`, `cold` | Proof duration per phase; `cold` for the first proof after a worker registered |
//! | `coordinator_last_job_duration_seconds`, `_steps`, `_instances`, `_mhz` | Gauge | `kind` | The last successful job |
//! | `coordinator_last_job_cold` | Gauge | `kind` | 1 if the last proof was the first after a worker registered |
//! | `coordinator_last_proof_phase_seconds` | Gauge | `phase` | The last proof's duration per phase |
//! | `coordinator_last_asm_seconds`, `coordinator_last_asm_mhz` | Gauge | `kind` | The last worker's assembly execution |
//! | `coordinator_last_witness_seconds` | Gauge | `kind` | The last worker's witness computation |
//! | `coordinator_last_executor_seconds` | Gauge | `kind`, `part` | The last worker's executor: `total`, `execution`, `count_and_plan`, `count_and_plan_mo` |
//! | `coordinator_dispatch_delay_seconds` | Histogram | `kind` | From starting a phase to a worker receiving its task |
//! | `coordinator_last_dispatch_delay_seconds` | Gauge | `kind` | The last of those |
//! | `coordinator_last_proof_worker_phase_seconds` | Gauge | `phase`, `worker_id` | Each worker's time over phases 1 and 2 of the last proof |
//! | `coordinator_proof_worker_spread_seconds` | Histogram | `phase` | Slowest minus fastest worker over phases 1 and 2, for proofs on several workers |
//! | `coordinator_job_cost_total` | Counter | `kind`, `type` | Cost of the jobs that succeeded, by type (main, opcode, memory, precompile, tables, other) |
//! | `coordinator_last_job_cost` | Gauge | `kind`, `type` | The last successful job's cost, by type |
//! | `coordinator_setups_total` | Counter | `outcome` | Program setups the workers acknowledged |
//! | `coordinator_worker_registrations_total` | Counter | `reconnect` | Worker registrations |
//! | `coordinator_worker_disconnects_total` | Counter | — | Worker disconnections |
//! | `coordinator_worker_recoveries_total` | Counter | `result` | Workers back from a cancelled or failed job (`done`) or given up on (`expired`) |
//! | `coordinator_worker_recovery_seconds` | Histogram | — | From parking a worker to its recovery |
//! | `coordinator_last_worker_recovery_seconds` | Gauge | — | The last recovery |
//!
//! Read from the coordinator's state when scraped:
//!
//! | Name | Type | Labels | Description |
//! |------|------|--------|-------------|
//! | `coordinator_workers_connected` | Gauge | — | Workers not disconnected |
//! | `coordinator_compute_units` | Gauge | `kind` | Compute units: `total` (connected) and `available` (ready) |
//! | `coordinator_worker_state` | Gauge | `worker_id` | 0 disconnected, 1 ready, 2 setting up, 3 computing, 4 recovering, 5 error |
//! | `coordinator_registered_programs_total` | Gauge | — | Programs in the coordinator's ELF cache |
//! | `coordinator_program_registered_timestamp_seconds` | Gauge | `hash_id`, `bytes` | When each program was registered |
//! | `coordinator_last_execution_air_instances` | Gauge | `air` | Instances per AIR in the last execution |
//! | `coordinator_job_now_seconds` | Gauge | `job_id`, `kind`, `status`, `phase`, `workers`, `mhz`, `ended_at` | The jobs in flight, or the one that just ended |
//! | `coordinator_recent_job` | Gauge | `job_id`, `kind`, `outcome`, `finished`, `duration_seconds`, `phases`, `steps`, `instances`, `mhz`, `input_bytes` | The last jobs, valued at their end time |
//! | `coordinator_recent_event` | Gauge | `seq`, `at`, `kind`, `detail` | The last events, valued at their time |
//! | `coordinator_job_failure_info` | Gauge | `seq`, `at`, `job_id`, `reason`, `cause`, `input_bytes`, `steps` | The last failures, valued at their time |
//! | `coordinator_proof_seconds` | Gauge, timestamped | `phase` | Each proof's phases and total, at the time it ended |
//! | `coordinator_job_unsuccessful_seconds` | Gauge, timestamped | `outcome` | Each failed or cancelled job's duration, at the time it ended |
//! | `coordinator_proof_input_bytes`, `coordinator_proof_steps` | Gauge, timestamped | — | Each proof's input size and steps, at the same time as its `coordinator_proof_seconds` |
//! | `process_resident_memory_bytes`, `process_cpu_seconds_total`, `process_start_time_seconds` | — | — | This process |
//!
//! The timestamped series take their next queued sample on each scrape: point a single
//! Prometheus at a coordinator.
//!
//! ## Scrape endpoint
//!
//! When metrics are enabled, a lightweight HTTP server is started on
//! `metrics.host:metrics.port` (default `0.0.0.0:9090`).
//!
//! - `GET /metrics` → Prometheus text format
//! - `GET /health`  → `200 OK` (liveness probe)

use std::sync::{Arc, OnceLock};

use anyhow::Result;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use zisk_cluster_common::prometheus_text;

use crate::backend::BackendService;
use crate::config::{MetricsConfig, ServiceConfig};

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the Prometheus global recorder and stash the handle for `/metrics`
/// rendering. Must be called once at startup before any metric is recorded.
pub fn install_prometheus() -> Result<()> {
    // Buckets sized for ~10s jobs (extra headroom for slow outliers).
    const JOB_DURATION_BUCKETS: &[f64] = &[1.0, 2.0, 5.0, 10.0, 20.0, 60.0, 300.0];
    // A phase of a proof: sub-second to minutes.
    const PROOF_PHASE_BUCKETS: &[f64] =
        &[0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 60.0, 120.0, 300.0, 600.0];
    // A healthy recovery is sub-second; a stuck one runs to the recovery timeout.
    const RECOVERY_BUCKETS: &[f64] =
        &[0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0];
    // A task reaches its worker in milliseconds unless the worker is busy elsewhere.
    const DISPATCH_BUCKETS: &[f64] =
        &[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0];
    // Workers of one proof finish a phase within a fraction of it of each other.
    const SPREAD_BUCKETS: &[f64] =
        &[0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0];

    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("coordinator_job_duration_seconds".to_owned()),
            JOB_DURATION_BUCKETS,
        )?
        .set_buckets_for_metric(
            Matcher::Full("coordinator_proof_phase_seconds".to_owned()),
            PROOF_PHASE_BUCKETS,
        )?
        .set_buckets_for_metric(
            Matcher::Full("coordinator_worker_recovery_seconds".to_owned()),
            RECOVERY_BUCKETS,
        )?
        .set_buckets_for_metric(
            Matcher::Full("coordinator_dispatch_delay_seconds".to_owned()),
            DISPATCH_BUCKETS,
        )?
        .set_buckets_for_metric(
            Matcher::Full("coordinator_proof_worker_spread_seconds".to_owned()),
            SPREAD_BUCKETS,
        )?
        .install_recorder()?;
    PROMETHEUS_HANDLE.set(handle).ok(); // ignore if already set (e.g. in tests)
    register_descriptions();
    Ok(())
}

fn register_descriptions() {
    metrics::describe_gauge!(
        "coordinator_info",
        "Always 1, labelled with the coordinator's configured name, environment and version"
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
        "coordinator_api_connections",
        "Open connections to the coordinator's API, by client address"
    );
    metrics::describe_gauge!(
        "coordinator_active_jobs",
        "Number of currently active (non-terminal) jobs"
    );
    metrics::describe_counter!(
        "coordinator_jobs_total",
        "Jobs that ended, labelled by kind and final outcome; failures also by reason class \
         (timeout, worker, other)"
    );
    metrics::describe_gauge!(
        "coordinator_registered_programs_total",
        "Number of registered guest programs (ELFs in the coordinator's cache)"
    );
    metrics::describe_gauge!(
        "coordinator_workers_connected",
        "Number of workers in the coordinator's pool that are not disconnected"
    );
    metrics::describe_gauge!(
        "coordinator_compute_units",
        "Compute units of the connected workers (total) and of the ready ones (available)"
    );
    metrics::describe_counter!(
        "coordinator_worker_jobs_total",
        "Per-worker participation count, labelled by worker_id and final outcome"
    );
    metrics::describe_histogram!(
        "coordinator_job_duration_seconds",
        "End-to-end job duration (Contributions phase start → terminal state) in seconds"
    );
    metrics::describe_counter!("coordinator_job_steps_total", "Steps of the jobs that succeeded");
    metrics::describe_histogram!(
        "coordinator_proof_phase_seconds",
        "Successful proofs' duration per phase (1 contributions, 2 prove, 3 aggregation); \
         cold for the first proof after a worker registered"
    );
    metrics::describe_gauge!(
        "coordinator_last_job_duration_seconds",
        "The last successful job's duration"
    );
    metrics::describe_gauge!("coordinator_last_job_steps", "The last successful job's steps");
    metrics::describe_gauge!(
        "coordinator_last_job_instances",
        "The last successful job's AIR instances"
    );
    metrics::describe_gauge!(
        "coordinator_last_job_mhz",
        "The last successful job's steps per second of its duration, in millions"
    );
    metrics::describe_gauge!(
        "coordinator_last_job_cold",
        "1 if the last successful proof was the first after a worker registered"
    );
    metrics::describe_gauge!(
        "coordinator_last_proof_phase_seconds",
        "The last successful proof's duration per phase"
    );
    metrics::describe_gauge!(
        "coordinator_last_asm_seconds",
        "The last worker-reported assembly execution time"
    );
    metrics::describe_gauge!(
        "coordinator_last_asm_mhz",
        "The last worker-reported assembly execution speed"
    );
    metrics::describe_gauge!(
        "coordinator_last_witness_seconds",
        "The last worker-reported witness computation time"
    );
    metrics::describe_gauge!(
        "coordinator_last_executor_seconds",
        "The last worker-reported executor time, by part (total, execution, count_and_plan, \
         count_and_plan_mo)"
    );
    metrics::describe_histogram!(
        "coordinator_dispatch_delay_seconds",
        "From the coordinator starting a phase to a worker receiving its task, in seconds"
    );
    metrics::describe_gauge!(
        "coordinator_last_dispatch_delay_seconds",
        "The last delay from starting a phase to a worker receiving its task"
    );
    metrics::describe_gauge!(
        "coordinator_last_proof_worker_phase_seconds",
        "Each worker's time over phases 1 (contributions) and 2 (prove) of the last proof"
    );
    metrics::describe_histogram!(
        "coordinator_proof_worker_spread_seconds",
        "The slowest minus the fastest worker's time over phases 1 and 2, for proofs on \
         several workers"
    );
    metrics::describe_counter!(
        "coordinator_job_cost_total",
        "Cost of the jobs that succeeded, by type (main, opcode, memory, precompile, tables, other)"
    );
    metrics::describe_gauge!(
        "coordinator_last_job_cost",
        "The last successful job's cost, by type"
    );
    metrics::describe_counter!(
        "coordinator_setups_total",
        "Program setups the workers acknowledged, by outcome"
    );
    metrics::describe_counter!(
        "coordinator_worker_registrations_total",
        "Worker registrations, new or reconnections"
    );
    metrics::describe_counter!("coordinator_worker_disconnects_total", "Worker disconnections");
    metrics::describe_counter!(
        "coordinator_worker_recoveries_total",
        "Workers parked after a cancelled or failed job that recovered (done) or were given up \
         on (expired)"
    );
    metrics::describe_histogram!(
        "coordinator_worker_recovery_seconds",
        "From parking a worker after a cancelled or failed job to its recovery, in seconds"
    );
    metrics::describe_gauge!(
        "coordinator_last_worker_recovery_seconds",
        "The last worker recovery's duration"
    );
}

/// Record who this coordinator is: `coordinator_info{name, environment, version} 1`.
pub fn record_info(service: &ServiceConfig) {
    metrics::gauge!(
        "coordinator_info",
        "name" => service.name.clone(),
        "environment" => service.environment.to_string(),
        "version" => service.version.clone()
    )
    .set(1.0);
}

/// Start the HTTP server that serves `/metrics` and `/health`.
/// No-ops if `cfg.enabled` is false.
/// The server stops accepting new connections when `cancel` is cancelled.
pub async fn start<B: BackendService>(
    cfg: &MetricsConfig,
    cancel: CancellationToken,
    backend: Arc<B>,
) -> Result<()> {
    if !cfg.enabled {
        return Ok(());
    }

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            warn!("metrics server failed to bind {addr}: {e} — continuing without metrics");
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
                    tokio::spawn(serve_connection(stream, Arc::clone(&backend)));
                }
            }
        }
    });

    Ok(())
}

async fn serve_connection<B: BackendService>(mut stream: tokio::net::TcpStream, backend: Arc<B>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or("");

    let response = if first_line.starts_with("GET /metrics") {
        if let Some(handle) = PROMETHEUS_HANDLE.get() {
            // The backend's text first: it sets the gauges the recorder renders.
            let state = backend.render_metrics().await;
            let mut body = handle.render();
            body.push_str(&state);
            prometheus_text::process_metrics(&mut body);
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
        } else {
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n".to_owned()
        }
    } else if first_line.starts_with("GET /health") {
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK".to_owned()
    } else {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_owned()
    };

    let _ = stream.write_all(response.as_bytes()).await;
}
