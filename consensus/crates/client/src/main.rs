mod prover_service;

use anyhow::Result;
use clap::Parser;
use consensus_core::ProverId;
use prover_service::{ProverConfig, ProverService};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "consensus-client")]
#[command(about = "A prover client for the Consensus Network")]
#[command(version)]
struct Cli {
    /// Server URL
    #[arg(short, long)]
    url: String,

    /// Prover ID (defaults to auto-generated UUID)
    #[arg(long)]
    prover_id: Option<String>,
    // /// Number of CPU cores to advertise
    // #[arg(long, default_value_t = num_cpus::get() as u32)]
    // cpu_cores: u32,

    // /// Number of GPUs to advertise
    // #[arg(long, default_value = "0")]
    // gpu_count: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    consensus_core::tracing::init()?;

    let cli = Cli::parse();

    info!("Starting prover client");

    let prover_id = cli.prover_id.map(ProverId::from).unwrap_or_else(ProverId::new);

    let config = ProverConfig {
        prover_id,
        server_address: cli.url,
        reconnect_interval_seconds: 5,
        heartbeat_timeout_seconds: 30,
        compute_capacity: consensus_api::ComputeCapacity { compute_units: 0 },
    };

    info!("Prover ID: {}", config.prover_id);
    info!("Server: {}", config.server_address);
    info!("Compute Capacity: {} units", config.compute_capacity.compute_units);

    let mut client = ProverService::new(config);

    if let Err(e) = client.run().await {
        error!("Prover client error: {}", e);
        return Err(e);
    }

    Ok(())
}
