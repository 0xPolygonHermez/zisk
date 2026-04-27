use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::info;
use zisk_coordinator_api::dto::{
    DomainInputChunk, DomainInputKind, DomainJobKind, DomainJobKindResponse, DomainProofKind,
    DomainProveRequest, DomainSetupRequest, RegisterGuestProgramRequestDto, TerminalStatus,
};
use zisk_coordinator_client::CoordinatorClient;

#[derive(Parser)]
#[command(name = "zisk-prove-client", about = "Submit prove jobs to the ZisK coordinator")]
struct Cli {
    /// Coordinator gRPC URL
    #[arg(long, default_value = "http://localhost:7000", env = "ZISK_COORDINATOR_URL")]
    coordinator: String,

    /// Connection timeout in seconds
    #[arg(long, default_value_t = 10)]
    connect_timeout: u64,

    /// Per-request timeout in seconds (0 = no timeout)
    #[arg(long, default_value_t = 3600)]
    request_timeout: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register an ELF binary and print its hash_id
    Register {
        /// Path to the ZisK ELF binary
        #[arg(short, long)]
        elf: PathBuf,
    },

    /// Run setup for a registered program (must be done once before proving)
    Setup {
        /// Path to the ZisK ELF binary (registers it automatically if needed)
        #[arg(short, long)]
        elf: PathBuf,

        /// Enable hints support for this program
        #[arg(long, default_value_t = false)]
        with_hints: bool,
    },

    /// Generate a proof for a registered and set-up program (run `setup` first)
    Prove {
        /// hash_id of the registered program (from `register` or `setup`)
        #[arg(short = 'H', long)]
        hash_id: String,

        /// Input data file for the guest program
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Hints data file for the guest program
        #[arg(long)]
        hints: Option<PathBuf>,

        /// Proof type to generate
        #[arg(long, default_value = "stark", value_parser = parse_proof_kind)]
        proof: DomainProofKind,

        /// Save the proof bytes to this file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Proof timeout in seconds (0 = no timeout)
        #[arg(long, default_value_t = 0)]
        proof_timeout: u64,
    },
}

fn parse_proof_kind(s: &str) -> Result<DomainProofKind, String> {
    match s.to_lowercase().as_str() {
        "stark" => Ok(DomainProofKind::Stark),
        "stark-minimal" | "stark_minimal" | "minimal" => Ok(DomainProofKind::StarkMinimal),
        "plonk" => Ok(DomainProofKind::Plonk),
        other => Err(format!("unknown proof kind '{other}'; use stark, stark-minimal, or plonk")),
    }
}

fn connect(cli: &Cli) -> Result<CoordinatorClient> {
    let connect_timeout = Duration::from_secs(cli.connect_timeout);
    let request_timeout = if cli.request_timeout == 0 {
        Duration::from_secs(u64::MAX / 2)
    } else {
        Duration::from_secs(cli.request_timeout)
    };
    CoordinatorClient::connect(cli.coordinator.clone(), connect_timeout, request_timeout)
        .with_context(|| format!("Failed to connect to coordinator at {}", cli.coordinator))
}

/// Register the ELF and return its hash_id.
fn register_elf(client: &CoordinatorClient, elf_path: &PathBuf) -> Result<String> {
    let elf_bytes = std::fs::read(elf_path)
        .with_context(|| format!("Cannot read ELF: {}", elf_path.display()))?;
    info!("Registering ELF ({} bytes) …", elf_bytes.len());
    let req = RegisterGuestProgramRequestDto { zisk_elf: elf_bytes };
    let hash_id = client.register_program(req.zisk_elf)?;
    info!("Registered. hash_id = {hash_id}");
    Ok(hash_id)
}

/// Run setup for the given hash_id and wait until it completes.
fn run_setup(client: &CoordinatorClient, hash_id: &str, with_hints: bool) -> Result<()> {
    info!("Running setup for hash_id = {hash_id}, with_hints = {with_hints} …");
    let job = client.submit_job(DomainJobKind::Setup(DomainSetupRequest {
        hash_id: hash_id.to_string(),
        with_hints,
    }))?;
    info!("Setup job submitted. job_id = {}", job.job_id());
    match job.wait(None)? {
        TerminalStatus::Completed(_) => {
            info!("Setup completed successfully.");
            Ok(())
        }
        TerminalStatus::Failed(f) => {
            anyhow::bail!("Setup job failed: {f:?}");
        }
        TerminalStatus::Cancelled => {
            anyhow::bail!("Setup job was cancelled.");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let client = connect(&cli)?;

    match &cli.command {
        Commands::Register { elf } => {
            let hash_id = register_elf(&client, elf)?;
            println!("{hash_id}");
        }

        Commands::Setup { elf, with_hints } => {
            let hash_id = register_elf(&client, elf)?;
            run_setup(&client, &hash_id, *with_hints)?;
        }

        Commands::Prove { hash_id, input, hints, proof, output, proof_timeout } => {
            // Build input
            let input_kind = match input {
                Some(path) => {
                    let data = std::fs::read(path)
                        .with_context(|| format!("Cannot read input: {}", path.display()))?;
                    info!("Using inline input from {} ({} bytes)", path.display(), data.len());
                    DomainInputKind::Inline(DomainInputChunk { data })
                }
                None => {
                    info!("No input provided — using empty input.");
                    DomainInputKind::Inline(DomainInputChunk { data: vec![] })
                }
            };

            let hints_kind = match hints {
                Some(path) => {
                    let data = std::fs::read(path)
                        .with_context(|| format!("Cannot read hints: {}", path.display()))?;
                    info!("Using inline hints from {} ({} bytes)", path.display(), data.len());
                    Some(DomainInputKind::Inline(DomainInputChunk { data }))
                }
                None => None,
            };

            let proof_timeout_opt = if *proof_timeout == 0 {
                None
            } else {
                Some(zisk_coordinator_api::dto::deadline_from_now(Duration::from_secs(
                    *proof_timeout,
                )))
            };

            // Submit prove job
            info!("Submitting prove job (proof_dest = {proof:?}) …");
            let job = client.submit_job(DomainJobKind::Prove(DomainProveRequest {
                hash_id: hash_id.clone(),
                input: input_kind,
                hints: hints_kind,
                proof_dest: proof.clone(),
                proof_timeout: proof_timeout_opt,
            }))?;
            let prove_job_id = job.job_id();
            info!("Prove job submitted. job_id = {prove_job_id}");

            // Live progress via watch (best-effort background task)
            let _watch = job.spawn_watch(|event| {
                info!("Job event: {event:?}");
                false // keep watching
            });

            // Wait for completion
            info!("Waiting for proof …");
            match job.wait(None)? {
                TerminalStatus::Completed(DomainJobKindResponse::Prove { proof: p, stats }) => {
                    info!(
                        steps = stats.steps,
                        duration_ms = stats.duration_nanos / 1_000_000,
                        "Prove completed."
                    );
                    info!(
                        proof_id = %p.proof_id,
                        kind     = ?p.proof_kind,
                        bytes    = p.data.len(),
                        "Proof received."
                    );

                    let out_path = output
                        .clone()
                        .unwrap_or_else(|| PathBuf::from(format!("{prove_job_id}.proof.bin")));
                    std::fs::write(&out_path, &p.data)
                        .with_context(|| format!("Cannot write proof to {}", out_path.display()))?;
                    println!("Proof saved to {}", out_path.display());

                    if !p.public_inputs.is_empty() {
                        let pi_path = out_path.with_extension("public_inputs.bin");
                        std::fs::write(&pi_path, &p.public_inputs).with_context(|| {
                            format!("Cannot write public inputs to {}", pi_path.display())
                        })?;
                        println!("Public inputs saved to {}", pi_path.display());
                    }
                }
                TerminalStatus::Completed(other) => {
                    anyhow::bail!("Unexpected job kind response: {other:?}");
                }
                TerminalStatus::Failed(f) => {
                    anyhow::bail!("Prove job failed: {f:?}");
                }
                TerminalStatus::Cancelled => {
                    anyhow::bail!("Prove job was cancelled.");
                }
            }
        }
    }

    Ok(())
}
