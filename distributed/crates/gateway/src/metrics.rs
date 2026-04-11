//! Prometheus metrics registration and HTTP scrape endpoint.
//!
//! ## Exposed metrics
//!
//! | Name | Type | Labels | Description |
//! |------|------|--------|-------------|
//! | `gateway_requests_total` | Counter | `method`, `status` | gRPC calls by method and outcome |
//! | `gateway_request_duration_seconds` | Histogram | `method` | gRPC call latency |
//! | `gateway_active_jobs` | Gauge | — | Currently active (non-terminal) jobs |
//! | `gateway_jobs_total` | Counter | `kind`, `outcome` | Jobs by kind and final outcome |
//! | `gateway_registered_programs_total` | Gauge | — | Registered guest programs |
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
use tracing::info;

use crate::config::MetricsConfig;

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the Prometheus global recorder and stash the handle for `/metrics`
/// rendering. Must be called once at startup before any metric is recorded.
pub fn install_prometheus() -> Result<()> {
    let handle = PrometheusBuilder::new().install_recorder()?;
    PROMETHEUS_HANDLE.set(handle).ok(); // ignore if already set (e.g. in tests)
    register_descriptions();
    Ok(())
}

fn register_descriptions() {
    metrics::describe_counter!(
        "gateway_requests_total",
        "Total gRPC calls by method and outcome (ok/error)"
    );
    metrics::describe_histogram!(
        "gateway_request_duration_seconds",
        "gRPC call latency in seconds"
    );
    metrics::describe_gauge!(
        "gateway_active_jobs",
        "Number of currently active (non-terminal) jobs"
    );
    metrics::describe_counter!(
        "gateway_jobs_total",
        "Jobs submitted, labelled by kind and final outcome"
    );
    metrics::describe_gauge!(
        "gateway_registered_programs_total",
        "Number of registered guest programs"
    );
}

/// Start the HTTP server that serves `/metrics` and `/health`.
/// No-ops if `cfg.enabled` is false.
pub async fn start(cfg: &MetricsConfig) -> Result<()> {
    if !cfg.enabled {
        return Ok(());
    }

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("metrics server listening on http://{addr}/metrics");

    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            tokio::spawn(serve_connection(stream));
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
