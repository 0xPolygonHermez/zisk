use anyhow::Result;
use clap::Parser;

mod handler_prove_block;
mod handler_server;

#[derive(Parser, Debug)]
#[command(name = "coordinator-server")]
#[command(about = "A Coordinator Network gRPC Server")]
struct CoordinatorServerArgs {
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
    ///   coordinator --webhook-url 'http://example.com/notify?job_id={$job_id}'
    ///   # becomes 'http://example.com/notify?job_id=12345'
    ///   coordinator --webhook-url 'http://example.com/notify'
    ///   # becomes 'http://example.com/notify/12345'
    #[arg(long, help = "Webhook URL for job finish notifications")]
    webhook_url: Option<String>,

    #[command(subcommand)]
    pub command: Option<CoordinatorServerCommands>,
}

#[derive(Parser, Debug)]
enum CoordinatorServerCommands {
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
        Some(CoordinatorServerCommands::ProveBlock {
            url,
            input,
            compute_capacity,
            simulated_node,
        }) => {
            // Prove block command
            handler_prove_block::handle(url, input, compute_capacity, simulated_node).await
        }
        None => {
            // Default to server mode when no subcommand is provided
            handler_server::handle(args.port, args.webhook_url).await
        }
    }
}
