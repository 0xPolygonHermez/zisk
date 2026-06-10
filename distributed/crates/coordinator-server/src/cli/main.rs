//! `zisk-coordinator` binary entry point.

use anyhow::{Context, Result};
use clap::Parser;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::{error, info, warn};
use zisk_cluster_api::{zisk_distributed_api_server::ZiskDistributedApiServer, MAX_MESSAGE_SIZE};
use zisk_cluster_common::init as init_logging;
use zisk_coordinator::{
    start_postgres_job_history, Config as CoordinatorConfig, Coordinator, CoordinatorGrpc,
    JobHistoryStore, PostgresJobHistoryOptions,
};

use zisk_coordinator_server::{
    backend::{
        coordinator::CoordinatorBackend, mock::MockBackend, BackendService, LiveStateProvider,
    },
    config::{BackendMode, Config},
    metrics, CoordinatorServer, HTTP2_CONNECTION_WINDOW_SIZE, HTTP2_STREAM_WINDOW_SIZE,
};

#[derive(Parser, Debug)]
#[command(name = "zisk-coordinator", about = "ZisK coordinator server", version)]
struct Args {
    /// Path to coordinator.toml configuration file.
    #[arg(
        long,
        env = "ZISK_COORDINATOR_CONFIG",
        help = "Path to coordinator.toml (overrides ZISK_COORDINATOR_CONFIG env var)"
    )]
    config: Option<String>,

    /// Override the external (client-facing) gRPC API port.
    #[arg(
        long,
        short,
        env = "ZISK_COORDINATOR_API_PORT",
        help = "External gRPC API port (client-facing)"
    )]
    api_port: Option<u16>,

    /// Override the internal cluster gRPC port (worker-facing).
    #[arg(
        long,
        env = "ZISK_COORDINATOR_CLUSTER_PORT",
        help = "Internal cluster gRPC port (worker-facing)"
    )]
    cluster_port: Option<u16>,

    /// Override the metrics port.
    #[arg(
        long,
        env = "ZISK_COORDINATOR_METRICS_PORT",
        value_name = "PORT",
        help = "Prometheus metrics port (default: 9090)"
    )]
    metrics_port: Option<u16>,

    /// Stable coordinator identity used as the Prometheus coordinator_id label.
    #[arg(
        long,
        env = "ZISK_COORDINATOR_ID",
        value_name = "ID",
        help = "Stable coordinator identity for metrics and history labels"
    )]
    coordinator_id: Option<String>,

    /// Override the log level.
    #[arg(
        long,
        env = "RUST_LOG",
        value_name = "LEVEL",
        help = "Log level: trace | debug | info | warn | error"
    )]
    log_level: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let cfg = Config::load(
        args.config,
        args.api_port,
        args.cluster_port,
        args.metrics_port,
        args.coordinator_id,
        args.log_level,
    )?;

    // Init logging (keep the guard alive for the process lifetime)
    let _log_guard = init_logging(Some(&cfg.logging), None)?;

    let scrape_token = resolve_scrape_token(
        &cfg.service.environment,
        cfg.metrics.enabled,
        std::env::var("ZISK_SCRAPE_TOKEN").ok(),
        std::env::var("ZISK_SCRAPE_TOKEN_FILE").ok(),
    )?;
    if cfg.metrics.enabled && scrape_token.is_none() {
        tracing::warn!(
            "ZISK_SCRAPE_TOKEN is unset; coordinator metrics endpoint accepts unauthenticated requests"
        );
    }
    zisk_coordinator_server::auth::set_expected_token(scrape_token);

    // Install Prometheus recorder before any metrics are recorded
    let started_at = chrono::Utc::now();
    metrics::install_prometheus(&cfg.service.coordinator_id)?;
    metrics::record_coordinator_info(&cfg.service.version, &cfg.service.environment.to_string());
    metrics::record_coordinator_start_time(started_at);

    let cancel = CancellationToken::new();

    match cfg.backend.mode {
        BackendMode::Mock => {
            let backend = MockBackend::new(cancel.clone());
            run(cfg, backend, None, cancel).await
        }
        BackendMode::Coordinator => {
            let coord_config = CoordinatorConfig::load(
                cfg.coordinator.config_file.clone(),
                Some(cfg.coordinator.port),
                None,
                cfg.coordinator.save_proofs,
                None,
            )?;
            let job_history = if cfg.job_history.enabled {
                // Env var takes precedence so credentials stay out of committed config.
                let database_url = std::env::var("ZISK_COORDINATOR_DATABASE_URL")
                    .ok()
                    .or_else(|| cfg.job_history.database_url.clone())
                    .context(
                        "job_history.enabled=true requires either the \
                         ZISK_COORDINATOR_DATABASE_URL env var or \
                         [job_history] database_url in the coordinator config",
                    )?;
                Some(
                    start_postgres_job_history(
                        &database_url,
                        PostgresJobHistoryOptions {
                            auto_migrate: cfg.job_history.auto_migrate,
                            channel_capacity: cfg.job_history.channel_capacity,
                            batch_size: cfg.job_history.batch_size,
                            flush_interval: std::time::Duration::from_millis(
                                cfg.job_history.flush_interval_ms,
                            ),
                        },
                    )
                    .await?,
                )
            } else {
                None
            };
            if let Some(store) = job_history.as_ref() {
                match store
                    .reconcile_interrupted_jobs(
                        &cfg.service.coordinator_id,
                        started_at,
                        "coordinator_restarted_mid_run",
                    )
                    .await
                {
                    Ok(0) => {}
                    Ok(count) => {
                        info!(count, "reconciled interrupted running job history rows");
                    }
                    Err(error) => {
                        warn!("failed to reconcile interrupted job history rows: {error:#}");
                    }
                }
                match store.last_successful_proof_timestamp(&cfg.service.coordinator_id).await {
                    Ok(timestamp) => metrics::record_last_successful_job_timestamp(timestamp),
                    Err(error) => {
                        warn!(
                            "failed to seed coordinator_last_successful_job_timestamp_seconds: {error:#}"
                        );
                        metrics::record_last_successful_job_timestamp(None);
                    }
                }
            } else {
                metrics::record_last_successful_job_timestamp(None);
            }
            let history_api = job_history.clone();
            let coordinator = Arc::new(match job_history {
                Some(store) => Coordinator::new_with_job_history(
                    coord_config,
                    cfg.service.coordinator_id.clone(),
                    store,
                ),
                None => Coordinator::new_with_coordinator_id(
                    coord_config,
                    cfg.service.coordinator_id.clone(),
                ),
            });

            // Pre-bind the worker-facing port at startup so we fail fast on conflicts.
            let worker_addr: std::net::SocketAddr =
                format!("0.0.0.0:{}", cfg.coordinator.port).parse()?;
            let worker_listener = TcpListener::bind(worker_addr).await?;

            tracing::info!("cluster coordinator listening on {addr}", addr = worker_addr);

            // Spawn the worker-facing gRPC server — shuts down when the cancel token fires.
            let worker_coordinator = Arc::clone(&coordinator);
            let cancel_worker = cancel.clone();
            tokio::spawn(async move {
                let svc = CoordinatorGrpc::from_arc(worker_coordinator);
                if let Err(e) = Server::builder()
                    .initial_connection_window_size(Some(HTTP2_CONNECTION_WINDOW_SIZE))
                    .initial_stream_window_size(Some(HTTP2_STREAM_WINDOW_SIZE))
                    .add_service(
                        ZiskDistributedApiServer::new(svc)
                            .max_decoding_message_size(MAX_MESSAGE_SIZE)
                            .max_encoding_message_size(MAX_MESSAGE_SIZE),
                    )
                    .serve_with_incoming_shutdown(
                        TcpListenerStream::new(worker_listener),
                        cancel_worker.cancelled_owned(),
                    )
                    .await
                {
                    error!("embedded coordinator worker gRPC server exited: {e:#}");
                }
            });

            let backend = CoordinatorBackend::new(coordinator);
            run(cfg, backend, history_api, cancel).await
        }
    }
}

async fn run<B: BackendService + LiveStateProvider>(
    cfg: Config,
    backend: B,
    history: Option<Arc<dyn JobHistoryStore>>,
    cancel: CancellationToken,
) -> Result<()> {
    let server = match history {
        Some(history) => CoordinatorServer::new_with_history(cfg, backend, history, cancel),
        None => CoordinatorServer::new(cfg, backend, cancel),
    };
    server.run().await.map_err(|e| {
        error!("coordinator server exited with error: {e:#}");
        e
    })
}

fn resolve_scrape_token(
    environment: &zisk_cluster_common::Environment,
    metrics_enabled: bool,
    token: Option<String>,
    token_file: Option<String>,
) -> Result<Option<String>> {
    let scrape_token = match normalize_token(token) {
        Some(token) => Some(token),
        None => read_scrape_token_file(token_file)?,
    };
    if metrics_enabled
        && matches!(environment, zisk_cluster_common::Environment::Production)
        && scrape_token.is_none()
    {
        anyhow::bail!(
            "ZISK_SCRAPE_TOKEN or ZISK_SCRAPE_TOKEN_FILE is required when metrics are enabled in production"
        );
    }
    Ok(scrape_token)
}

fn normalize_token(token: Option<String>) -> Option<String> {
    let token = token?;
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_owned())
}

fn read_scrape_token_file(path: Option<String>) -> Result<Option<String>> {
    let Some(path) = normalize_token(path) else {
        return Ok(None);
    };
    let token = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read ZISK_SCRAPE_TOKEN_FILE at {path}"))?;
    Ok(normalize_token(Some(token)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_cluster_common::Environment;

    #[test]
    fn production_metrics_require_scrape_token() {
        let error = resolve_scrape_token(&Environment::Production, true, None, None).unwrap_err();
        assert!(error.to_string().contains("ZISK_SCRAPE_TOKEN or"));
    }

    #[test]
    fn production_metrics_reject_empty_scrape_token() {
        let error = resolve_scrape_token(&Environment::Production, true, Some(String::new()), None)
            .unwrap_err();
        assert!(error.to_string().contains("ZISK_SCRAPE_TOKEN or"));
    }

    #[test]
    fn production_without_metrics_does_not_require_scrape_token() {
        assert_eq!(
            resolve_scrape_token(&Environment::Production, false, None, None).unwrap(),
            None
        );
    }

    #[test]
    fn development_metrics_can_run_without_scrape_token() {
        assert_eq!(
            resolve_scrape_token(&Environment::Development, true, None, None).unwrap(),
            None
        );
    }

    #[test]
    fn configured_scrape_token_is_preserved() {
        assert_eq!(
            resolve_scrape_token(&Environment::Production, true, Some("secret".to_owned()), None)
                .unwrap(),
            Some("secret".to_owned())
        );
    }

    #[test]
    fn scrape_token_file_is_used_when_token_is_unset() {
        let path = temp_token_path("file");
        std::fs::write(&path, "from-file\n").unwrap();
        let token = resolve_scrape_token(
            &Environment::Production,
            true,
            None,
            Some(path.to_string_lossy().into_owned()),
        )
        .unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(token, Some("from-file".to_owned()));
    }

    #[test]
    fn scrape_token_env_wins_over_file() {
        let path = temp_token_path("precedence");
        std::fs::write(&path, "from-file").unwrap();
        let token = resolve_scrape_token(
            &Environment::Production,
            true,
            Some("from-env".to_owned()),
            Some(path.to_string_lossy().into_owned()),
        )
        .unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(token, Some("from-env".to_owned()));
    }

    fn temp_token_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("zisk-scrape-token-{name}-{}", std::process::id()))
    }
}
