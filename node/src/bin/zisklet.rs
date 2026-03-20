use clap::Parser;
use tracing::info;
use zisk_node::{
    cluster::ClusterRegistry,
    config::NodeConfig,
    daemon::NodeServer,
    logging,
};

/// ZisK node daemon — manages coordinator/worker processes and exposes ZiskNodeApi.
#[derive(Parser, Debug)]
#[command(name = "zisklet", about = "ZisK node daemon")]
struct Args {
    /// Path to node configuration file (TOML)
    #[arg(long, env = "ZISK_NODE_CONFIG")]
    config: Option<String>,

    /// Override listening port
    #[arg(long, env = "ZISK_NODE_PORT")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config = NodeConfig::load(args.config, args.port)?;

    logging::init(&config.logging)?;

    info!("zisklet v{} starting", env!("CARGO_PKG_VERSION"));

    // Load cluster registry if a clusters file is configured (head node mode).
    let cluster_registry = if let Some(ref path) = config.node.clusters_file {
        match ClusterRegistry::load(path.clone()) {
            Ok(reg) => {
                info!("Loaded clusters file: {}", path.display());
                Some(reg)
            }
            Err(e) => {
                tracing::warn!("Could not load clusters file '{}': {e}. Running without cluster management.", path.display());
                None
            }
        }
    } else {
        None
    };

    NodeServer::new(config, cluster_registry).run().await?;
    Ok(())
}
