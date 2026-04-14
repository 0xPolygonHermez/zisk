//! `zisk-gateway` binary entry point.

use anyhow::Result;
use clap::Parser;
use std::{sync::Arc, time::Duration};
use tracing::error;
use zisk_distributed_common::init as init_logging;
use zisk_distributed_coordinator::{Coordinator, CoordinatorGrpc, Config as CoordinatorConfig};
use zisk_distributed_grpc_api::{zisk_distributed_api_server::ZiskDistributedApiServer, MAX_MESSAGE_SIZE};
use tonic::transport::Server;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

use zisk_gateway::{
    backend::{
        coordinator::CoordinatorBackend,
        embedded_coordinator::EmbeddedCoordinatorBackend,
        mock::MockBackend,
        BackendService,
    },
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

    match cfg.backend.mode {
        BackendMode::Mock => {
            let backend = MockBackend::new();
            run(cfg, backend).await
        }
        BackendMode::Coordinator => {
            let backend = CoordinatorBackend::new(
                cfg.coordinator.url.clone(),
                Duration::from_secs(cfg.coordinator.connect_timeout_seconds),
                Duration::from_secs(cfg.coordinator.request_timeout_seconds),
            )?;
            run(cfg, backend).await
        }
        BackendMode::Embedded => {
            let coord_config = CoordinatorConfig::load(
                cfg.embedded_coordinator.config_file.clone(),
                Some(cfg.embedded_coordinator.worker_port),
                None,
                false,
                false,
                None,
            )?;
            let coordinator = Arc::new(Coordinator::new(coord_config));

            // Pre-bind the worker-facing port at startup so we fail fast on conflicts.
            let worker_addr: std::net::SocketAddr =
                format!("0.0.0.0:{}", cfg.embedded_coordinator.worker_port).parse()?;
            let worker_listener = TcpListener::bind(worker_addr).await?;

            // Spawn the worker-facing gRPC server (ZiskDistributedApi) in the background.
            let worker_coordinator = Arc::clone(&coordinator);
            tokio::spawn(async move {
                let svc = CoordinatorGrpc::from_arc(worker_coordinator);
                if let Err(e) = Server::builder()
                    .add_service(
                        ZiskDistributedApiServer::new(svc)
                            .max_decoding_message_size(MAX_MESSAGE_SIZE)
                            .max_encoding_message_size(MAX_MESSAGE_SIZE),
                    )
                    .serve_with_incoming(TcpListenerStream::new(worker_listener))
                    .await
                {
                    error!("embedded coordinator worker gRPC server exited: {e:#}");
                }
            });

            let backend = EmbeddedCoordinatorBackend::new(coordinator);
            run(cfg, backend).await
        }
    }
}

async fn run<B: BackendService>(cfg: Config, backend: B) -> Result<()> {
    GatewayServer::new(cfg, backend).run().await.map_err(|e| {
        error!("gateway exited with error: {e:#}");
        e
    })
}
