mod handler_prove_block;
mod handler_server;
mod service;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "consensus-server")]
#[command(about = "A Consensus Network gRPC Server")]
struct ConsensusServerArgs {
    #[command(subcommand)]
    pub command: ConsensusServerCommands,
}

#[derive(Parser, Debug)]
enum ConsensusServerCommands {
    /// Start the gRPC server (default mode)
    Server {
        /// Port to bind the gRPC server to
        #[arg(short, long, help = "Port number for the gRPC server")]
        port: Option<u16>,
    },
    /// Prove a block with the specified input file and node
    ProveBlock {
        /// Server URL
        #[arg(short, long)]
        url: String,

        /// Path to the input file
        /// NOTE: THIS IS A DEV FEATURE IT WILL BE REMOVED IN PRODUCTION
        #[arg(long, help = "Path to the input file for block proving")]
        input: String,

        /// Number of provers needed to generate the block proof
        #[arg(long, help = "Number of provers needed to generate the block proof")]
        provers: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = ConsensusServerArgs::parse();

    // Initialize tracing
    consensus_core::tracing::init()?;

    match args.command {
        ConsensusServerCommands::Server { port } => {
            // Server mode
            handler_server::handle(port).await
        }
        ConsensusServerCommands::ProveBlock { url, input, provers } => {
            // Initialize basic tracing for the prove-block command
            handler_prove_block::handle(url, input, provers as u32).await
        }
    }
}
