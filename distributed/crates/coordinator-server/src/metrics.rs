//! Prometheus metrics registration and HTTP scrape endpoint.
//!
//! ## Exposed metrics
//!
//! | Name | Type | Labels | Description |
//! |------|------|--------|-------------|
//! | `coordinator_requests_total` | Counter | `method`, `status` | gRPC calls by method and outcome |
//! | `coordinator_request_duration_seconds` | Histogram | `method` | gRPC call latency |
//! | `coordinator_active_jobs` | Gauge | — | Currently active (non-terminal) jobs |
//! | `coordinator_jobs_total` | Counter | `kind`, `outcome` | Jobs by kind and final outcome |
//! | `coordinator_registered_programs_total` | Gauge | — | Registered guest programs |
//! | `coordinator_workers_connected` | Gauge | — | Workers currently registered in the pool |
//! | `coordinator_worker_jobs_total` | Counter | `worker_id`, `outcome` | Per-worker participation count by job outcome |
//! | `coordinator_job_duration_seconds` | Histogram | `outcome` | End-to-end job duration (Contributions start → terminal state) |
//!
//! ## Scrape endpoint
//!
//! When metrics are enabled, a lightweight HTTP server is started on
//! `metrics.host:metrics.port` (default `0.0.0.0:9090`).
//!
//! - `GET /metrics` → Prometheus text format
//! - `GET /health`  → `200 OK` (liveness probe)

use std::sync::OnceLock;

use anyhow::Result;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::MetricsConfig;

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the Prometheus global recorder and stash the handle for `/metrics`
/// rendering. Must be called once at startup before any metric is recorded.
pub fn install_prometheus() -> Result<()> {
    // Buckets sized for ~10s jobs (extra headroom for slow outliers).
    const JOB_DURATION_BUCKETS: &[f64] = &[1.0, 2.0, 5.0, 10.0, 20.0, 60.0, 300.0];

    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "coordinator_job_duration_seconds".to_owned(),
            ),
            JOB_DURATION_BUCKETS,
        )?
        .install_recorder()?;
    PROMETHEUS_HANDLE.set(handle).ok(); // ignore if already set (e.g. in tests)
    register_descriptions();
    Ok(())
}

fn register_descriptions() {
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
        "Number of currently active (non-terminal) jobs"
    );
    metrics::describe_counter!(
        "coordinator_jobs_total",
        "Jobs submitted, labelled by kind and final outcome"
    );
    metrics::describe_gauge!(
        "coordinator_registered_programs_total",
        "Number of registered guest programs"
    );
    metrics::describe_gauge!(
        "coordinator_workers_connected",
        "Number of workers currently registered in the coordinator's pool"
    );
    metrics::describe_counter!(
        "coordinator_worker_jobs_total",
        "Per-worker participation count, labelled by worker_id and final outcome"
    );
    metrics::describe_histogram!(
        "coordinator_job_duration_seconds",
        "End-to-end job duration (Contributions phase start → terminal state) in seconds"
    );
}

/// Start the HTTP server that serves `/metrics` and `/health`.
/// No-ops if `cfg.enabled` is false.
/// The server stops accepting new connections when `cancel` is cancelled.
pub async fn start(cfg: &MetricsConfig, cancel: CancellationToken) -> Result<()> {
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
                    tokio::spawn(serve_connection(stream));
                }
            }
        }
    });

    Ok(())
}

async fn serve_connection(mut stream: tokio::net::TcpStream) {
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
            let body = handle.render();
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
