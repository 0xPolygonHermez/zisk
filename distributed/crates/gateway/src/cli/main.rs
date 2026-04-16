//! `zisk-gateway` binary entry point.

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::error;
use zisk_distributed_common::init as init_logging;
use zisk_distributed_coordinator::{Config as CoordinatorConfig, Coordinator, CoordinatorGrpc};
use zisk_distributed_grpc_api::{
    zisk_distributed_api_server::ZiskDistributedApiServer, MAX_MESSAGE_SIZE,
};

use zisk_gateway::{
    backend::{coordinator::CoordinatorBackend, mock::MockBackend, BackendService},
    config::{BackendMode, Config},
    metrics, GatewayServer,
};

#[derive(Parser, Debug)]
#[command(name = "zisk-gateway", about = "ZisK public API gateway", version)]
struct Args {
    /// Path to gateway.toml configuration file.
    #[arg(
        long,
        env = "ZISK_GATEWAY_CONFIG",
        help = "Path to gateway.toml (overrides ZISK_GATEWAY_CONFIG env var)"
    )]
    config: Option<String>,

    /// Override the gRPC listen port.
    #[arg(long, short, env = "ZISK_GATEWAY_PORT", help = "gRPC listen port")]
    port: Option<u16>,

    /// Override the log level.
    #[arg(
        long,
        env = "RUST_LOG",
        value_name = "LEVEL",
        help = "Log level: trace | debug | info | warn | error"
    )]
    log_level: Option<String>,

    /// Override the backend mode.
    #[arg(
        long,
        env = "ZISK_GATEWAY_BACKEND",
        value_name = "MODE",
        help = "Backend mode: mock | coordinator"
    )]
    backend: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let cfg = Config::load(args.config, args.port, args.log_level, args.backend)?;

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
                Some(cfg.coordinator.worker_port),
                None,
                false,
                false,
                None,
            )?;
            let coordinator = Arc::new(Coordinator::new(coord_config));

            // Pre-bind the worker-facing port at startup so we fail fast on conflicts.
            let worker_addr: std::net::SocketAddr =
                format!("0.0.0.0:{}", cfg.coordinator.worker_port).parse()?;
            let worker_listener = TcpListener::bind(worker_addr).await?;

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
    GatewayServer::new(cfg, backend, cancel).run().await.map_err(|e| {
        error!("gateway exited with error: {e:#}");
        e
    })
}
