//! `zisk-gateway` binary entry point.

use anyhow::Result;
use clap::Parser;
use tracing::error;
use zisk_distributed_common::init as init_logging;

use zisk_gateway::{
    backend::{mock::MockBackend, BackendService},
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
            anyhow::bail!(
                "backend.mode = 'coordinator' is not yet implemented (phase 2). \
                 Use --backend mock for development."
            );
        }
    }
}

async fn run<B: BackendService>(cfg: Config, backend: B) -> Result<()> {
    GatewayServer::new(cfg, backend).run().await.map_err(|e| {
        error!("gateway exited with error: {e:#}");
        e
    })
}
