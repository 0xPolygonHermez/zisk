use anyhow::Result;
use clap::Parser;

mod handler_coordinator;
mod handler_prove;

#[derive(Parser, Debug)]
#[command(name = "zisk-coordinator")]
#[command(about = "The Coordinator for the Distributed ZisK Network")]
struct ZiskCoordinatorArgs {
    /// Path to configuration file
    #[arg(
        long,
        help = "Path to configuration file (overrides ZISK_COORDINATOR_CONFIG_PATH environment variable if exists)"
    )]
    config: Option<String>,

    /// Port where the ZisK Coordinator gRPC server will listen for incoming connections.
    #[arg(short, long, help = "Port number to bind the ZisK Coordinator gRPC server to")]
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
    pub command: Option<ZiskCoordinatorCommands>,
}

#[derive(Parser, Debug)]
enum ZiskCoordinatorCommands {
    /// Prove a block with the specified input file and node
    Prove {
        /// Coordinator URL
        #[arg(short, long)]
        coordinator_url: Option<String>,

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
    let args = ZiskCoordinatorArgs::parse();

    match args.command {
        Some(ZiskCoordinatorCommands::Prove {
            coordinator_url,
            input,
            compute_capacity,
            simulated_node,
        }) => {
            // Run the "prove" subcommand
            handler_prove::handle(coordinator_url, input, compute_capacity, simulated_node).await
        }
        None => {
            // No subcommand was provided → default to coordinator mode
            handler_coordinator::handle(args.config, args.port, args.webhook_url).await
        }
    }
}
