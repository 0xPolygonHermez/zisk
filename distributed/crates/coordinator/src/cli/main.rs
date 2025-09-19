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

        /// Webhook URL to notify when a job finishes.
        ///
        /// The placeholder `{$job_id}` can be used in the URL and will be
        /// replaced by the finished job ID.
        /// If the placeholder is not present, the coordinator automatically
        /// appends `/{job_id}` to the end of the URL.
        ///
        /// Examples:
        ///   coordinator server --webhook-url 'http://example.com/notify?job_id={$job_id}'
        ///   coordinator server --webhook-url 'http://example.com/notify'
        ///   # becomes 'http://example.com/notify/{job_id}'
        #[arg(long, help = "Webhook URL for job finish notifications")]
        webhook_url: Option<String>,
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
        #[arg(long, short, help = "Compute capacity needed to generate the block proof")]
        compute_capacity: u32,

        /// Compute capacity needed to aggregate
        #[arg(long, short, help = "Minimal compute capacity needed to aggregate proofs")]
        aggregate_compute_capacity: Option<u32>,

        #[arg(long, help = "Simulated node ID")]
        simulated_node: Option<u32>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = CoordinatorServerArgs::parse();

    // Initialize tracing
    distributed_common::tracing::init()?;

    match args.command {
        CoordinatorServerCommands::Server { port, webhook_url } => {
            // Server mode
            handler_server::handle(port, webhook_url).await
        }
        CoordinatorServerCommands::ProveBlock {
            url,
            input,
            compute_capacity,
            simulated_node,
            aggregate_compute_capacity,
        } => {
            // Initialize basic tracing for the prove-block command
            handler_prove_block::handle(
                url,
                input,
                compute_capacity,
                aggregate_compute_capacity,
                simulated_node,
            )
            .await
        }
    }
}
