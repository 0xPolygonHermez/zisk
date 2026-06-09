//! `zisk-coordinator` binary entry point.

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::error;
use zisk_cluster_api::{zisk_distributed_api_server::ZiskDistributedApiServer, MAX_MESSAGE_SIZE};
use zisk_cluster_common::init as init_logging;
use zisk_coordinator::{Config as CoordinatorConfig, Coordinator, CoordinatorGrpc};

use zisk_coordinator_server::{
    backend::{coordinator::CoordinatorBackend, mock::MockBackend, BackendService},
    config::{BackendMode, Config},
    metrics, CoordinatorServer,
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
        args.log_level,
    )?;

    // Init logging (keep the guard alive for the process lifetime)
    let _log_guard = init_logging(Some(&cfg.logging), None)?;

    // Install Prometheus recorder before any metrics are recorded
    metrics::install_prometheus()?;

    let cancel = CancellationToken::new();

    match cfg.backend.mode {
        BackendMode::Mock => {
            let backend = MockBackend::new(cancel.clone());
            run(cfg, backend, cancel).await
        }
        BackendMode::Coordinator => {
            let coord_config = CoordinatorConfig::load(
                cfg.coordinator.config_file.clone(),
                Some(cfg.coordinator.port),
                None,
                cfg.coordinator.save_proofs,
                None,
            )?;
            let coordinator = Arc::new(Coordinator::new(coord_config));

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
            run(cfg, backend, cancel).await
        }
    }
}

async fn run<B: BackendService>(cfg: Config, backend: B, cancel: CancellationToken) -> Result<()> {
    CoordinatorServer::new(cfg, backend, cancel).run().await.map_err(|e| {
        error!("coordinator server exited with error: {e:#}");
        e
    })
}
