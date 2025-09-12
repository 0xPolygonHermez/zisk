use anyhow::Result;
use clap::Parser;

mod handler_prove_block;
mod handler_server;

#[derive(Parser, Debug)]
#[command(name = "coordinator-server")]
#[command(about = "A Coordinator Network gRPC Server")]
struct CoordinatorServerArgs {
    #[command(subcommand)]
    pub command: CoordinatorServerCommands,
}

#[derive(Parser, Debug)]
enum CoordinatorServerCommands {
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

        /// Compute capacity needed to generate the block proof
        #[arg(long, help = "Compute capacity needed to generate the block proof")]
        compute_capacity: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = CoordinatorServerArgs::parse();

    // Initialize tracing
    distributed_common::tracing::init()?;

    match args.command {
        CoordinatorServerCommands::Server { port } => {
            // Server mode
            handler_server::handle(port).await
        }
        CoordinatorServerCommands::ProveBlock { url, input, compute_capacity } => {
            // Initialize basic tracing for the prove-block command
            handler_prove_block::handle(url, input, compute_capacity as u32).await
        }
    }
}
