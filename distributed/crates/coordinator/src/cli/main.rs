use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod handler_coordinator;
mod handler_prove;

#[derive(Parser, Debug)]
#[command(name = "zisk-coordinator")]
#[command(about = "The Coordinator for the Distributed ZisK Network")]
#[command(version)]
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

    /// Directory where to save generated proofs
    #[arg(long, help = "Directory to save generated proofs", conflicts_with = "no_save_proof")]
    proofs_dir: Option<PathBuf>,

    /// Disable saving proofs
    #[arg(
        long,
        help = "Do not save proofs",
        conflicts_with = "proofs_dir",
        default_value_t = false
    )]
    no_save_proofs: bool,

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
    /// Generate a proof with the specified input file and node
    Prove {
        /// Coordinator URL
        #[arg(long)]
        coordinator_url: Option<String>,

        /// Proof id
        #[arg(long, help = "ID of the proof to generate")]
        data_id: Option<String>,

        /// Path to the input file
        #[arg(long, help = "Path to the input file for proof generation")]
        input: Option<PathBuf>,

        /// Whether to send the input data directly
        #[clap(short = 'x', long, default_value_t = false)]
        direct_inputs: bool,

        /// Compute capacity needed to generate the proof
        #[arg(long, short, help = "Compute capacity needed to generate the proof")]
        compute_capacity: u32,

        #[arg(long, short, help = "Minimal compute capacity needed to generate the proof")]
        minimal_compute_capacity: Option<u32>,

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
            data_id,
            input,
            direct_inputs,
            compute_capacity,
            minimal_compute_capacity,
            simulated_node,
        }) => {
            // Run the "prove" subcommand
            handler_prove::handle(
                coordinator_url,
                data_id,
                input,
                direct_inputs,
                compute_capacity,
                simulated_node,
                minimal_compute_capacity,
            )
            .await
        }
        None => {
            // No subcommand was provided → default to coordinator mode
            handler_coordinator::handle(
                args.config,
                args.port,
                args.proofs_dir,
                args.no_save_proofs,
                args.webhook_url,
            )
            .await
        }
    }
}
