use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::info;
use zisk_coordinator_api::dto::{
    deadline_from_now, DomainExecuteRequest, DomainInputChunk, DomainInputKind, DomainJobKind,
    DomainJobKindResponse, DomainProofKind, DomainProveRequest, DomainSetupRequest, TerminalStatus,
};
use zisk_coordinator_client::CoordinatorClient;

#[derive(Parser)]
#[command(name = "zisk-prove-client", about = "Submit jobs to the ZisK coordinator")]
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

        /// Generate setup for emulator only (skips ASM service startup).
        /// Programs set up this way support `execute` but not `prove`.
        #[arg(long, default_value_t = false)]
        emulator_only: bool,
    },

    /// Generate a proof for a registered and set-up program (run `setup` first)
    Prove {
        /// hash_id of the registered program (from `register` or `setup`)
        #[arg(short = 'H', long)]
        hash_id: String,

        /// Input data file for the guest program
        #[arg(short, long, conflicts_with = "hints")]
        input: Option<PathBuf>,

        /// Hints data file for the guest program
        #[arg(long, conflicts_with = "input")]
        hints: Option<PathBuf>,

        /// Proof type to generate
        #[arg(long, default_value = "stark", value_parser = parse_proof_kind)]
        proof: DomainProofKind,

        /// Save the proof bytes to this file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Proof timeout in seconds (0 = no timeout)
        #[arg(long, default_value_t = 0)]
        timeout: u64,
    },

    /// Execute a registered program without generating a proof
    Execute {
        /// hash_id of the registered program (from `register` or `setup`)
        #[arg(short = 'H', long)]
        hash_id: String,

        /// Input data file for the guest program
        #[arg(short, long, conflicts_with = "hints")]
        input: Option<PathBuf>,

        /// Hints data file for the guest program
        #[arg(long, conflicts_with = "input")]
        hints: Option<PathBuf>,

        /// Save the public outputs to this file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Execute timeout in seconds (0 = no timeout)
        #[arg(long, default_value_t = 0)]
        timeout: u64,
    },

    /// Cancel a running or queued job by its id
    Cancel {
        /// Job UUID printed at submission time
        #[arg(short = 'j', long)]
        job_id: uuid::Uuid,
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

fn register_elf(client: &CoordinatorClient, elf_path: &PathBuf) -> Result<(String, String)> {
    let elf_bytes = std::fs::read(elf_path)
        .with_context(|| format!("Cannot read ELF: {}", elf_path.display()))?;
    let program_name =
        elf_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
    info!("Registering ELF ({} bytes) …", elf_bytes.len());
    let hash_id = client.register_program(elf_bytes)?;
    info!("Registered. hash_id = {hash_id}");
    Ok((hash_id, program_name))
}

fn run_setup(
    client: &CoordinatorClient,
    hash_id: &str,
    program_name: String,
    with_hints: bool,
    emulator_only: bool,
) -> Result<()> {
    info!(
        "Running setup for hash_id = {hash_id}, with_hints = {with_hints}, emulator_only = {emulator_only} …"
    );
    let job = client.submit_job(DomainJobKind::Setup(DomainSetupRequest {
        hash_id: hash_id.to_string(),
        program_name,
        with_hints,
        emulator_only,
    }))?;
    info!("Setup job submitted. job_id = {}", job.job_id());
    match job.wait(None)? {
        TerminalStatus::Completed(_) => {
            info!("Setup completed successfully.");
            Ok(())
        }
        TerminalStatus::Failed(f) => anyhow::bail!("Setup job failed: {f:?}"),
        TerminalStatus::Cancelled => anyhow::bail!("Setup job was cancelled."),
    }
}

fn read_input(path: &PathBuf) -> Result<DomainInputKind> {
    let data =
        std::fs::read(path).with_context(|| format!("Cannot read input: {}", path.display()))?;
    info!("Using inline input from {} ({} bytes)", path.display(), data.len());
    Ok(DomainInputKind::Inline(DomainInputChunk { data }))
}

fn spawn_watch(job: &zisk_coordinator_client::Job) -> zisk_coordinator_client::WatchHandle {
    job.spawn_watch(|event| {
        use zisk_coordinator_api::dto::DomainJobEvent;
        match &event {
            DomainJobEvent::Queued(e) => info!(job_id = %e.job_id, "Job queued"),
            DomainJobEvent::Started(e) => info!(job_id = %e.job_id, "Job started"),
            DomainJobEvent::Progress(e) => {
                info!(job_id = %e.job_id, phase = ?e.phase, "Job progress")
            }
            DomainJobEvent::WaitingForInput(e) => {
                info!(job_id = %e.job_id, "Job waiting for input")
            }
            DomainJobEvent::Completed(e) => info!(job_id = %e.job_id, "Job completed"),
            DomainJobEvent::Cancelled(e) => info!(job_id = %e.job_id, "Job cancelled"),
            DomainJobEvent::Failed(e) => {
                info!(job_id = %e.job_id, failure = ?e.failure, "Job failed")
            }
        }
        false
    })
}

fn run_prove(
    client: &CoordinatorClient,
    hash_id: &str,
    input: Option<&PathBuf>,
    hints: Option<&PathBuf>,
    proof: &DomainProofKind,
    output: Option<&PathBuf>,
    timeout: u64,
) -> Result<()> {
    let input_kind = match input {
        Some(path) => read_input(path)?,
        None => {
            info!("No input provided — using empty input.");
            DomainInputKind::Inline(DomainInputChunk { data: vec![] })
        }
    };
    let hints_kind = match hints {
        Some(path) => Some(read_input(path)?),
        None => None,
    };
    let timeout_opt = (timeout != 0).then(|| deadline_from_now(Duration::from_secs(timeout)));

    info!("Submitting prove job (proof_dest = {proof:?}) …");
    let job = client.submit_job(DomainJobKind::Prove(DomainProveRequest {
        hash_id: hash_id.to_string(),
        input: input_kind,
        hints: hints_kind,
        proof_dest: proof.clone(),
        proof_timeout: timeout_opt,
    }))?;
    let job_id = job.job_id();
    info!("Prove job submitted. job_id = {job_id}");

    let _watch = spawn_watch(&job);

    info!("Waiting for proof …");
    match job.wait(None)? {
        TerminalStatus::Completed(DomainJobKindResponse::Prove { proof: p, stats }) => {
            info!(
                steps = stats.steps,
                duration_ms = stats.duration_nanos / 1_000_000,
                proof_id = %p.proof_id,
                kind = ?p.proof_kind,
                bytes = p.data.len(),
                "Prove completed."
            );

            let out_path =
                output.cloned().unwrap_or_else(|| PathBuf::from(format!("{job_id}.proof.bin")));
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

            Ok(())
        }
        TerminalStatus::Completed(other) => {
            anyhow::bail!("Unexpected job kind response: {other:?}")
        }
        TerminalStatus::Failed(f) => anyhow::bail!("Prove job failed: {f:?}"),
        TerminalStatus::Cancelled => anyhow::bail!("Prove job was cancelled."),
    }
}

fn run_execute(
    client: &CoordinatorClient,
    hash_id: &str,
    input: Option<&PathBuf>,
    hints: Option<&PathBuf>,
    output: Option<&PathBuf>,
    timeout: u64,
) -> Result<()> {
    let input_kind = match input {
        Some(path) => read_input(path)?,
        None => {
            info!("No input provided — using empty input.");
            DomainInputKind::Inline(DomainInputChunk { data: vec![] })
        }
    };
    let hints_kind = match hints {
        Some(path) => Some(read_input(path)?),
        None => None,
    };
    let timeout_opt = (timeout != 0).then(|| deadline_from_now(Duration::from_secs(timeout)));

    info!("Submitting execute job …");
    let job = client.submit_job(DomainJobKind::Execute(DomainExecuteRequest {
        hash_id: hash_id.to_string(),
        input: input_kind,
        hints: hints_kind,
        execute_timeout: timeout_opt,
    }))?;
    let job_id = job.job_id();
    info!("Execute job submitted. job_id = {job_id}");

    let _watch = spawn_watch(&job);

    info!("Waiting for execution …");
    match job.wait(None)? {
        TerminalStatus::Completed(DomainJobKindResponse::Execute { stats, public_outputs }) => {
            info!(
                steps = stats.steps,
                duration_ms = stats.duration_nanos / 1_000_000,
                "Execute completed."
            );

            if !public_outputs.is_empty() {
                let out_path = output
                    .cloned()
                    .unwrap_or_else(|| PathBuf::from(format!("{job_id}.public_outputs.bin")));
                std::fs::write(&out_path, &public_outputs)
                    .with_context(|| format!("Cannot write outputs to {}", out_path.display()))?;
                println!("Public outputs saved to {}", out_path.display());
            } else {
                println!("Execute completed (no public outputs).");
            }

            Ok(())
        }
        TerminalStatus::Completed(other) => {
            anyhow::bail!("Unexpected job kind response: {other:?}")
        }
        TerminalStatus::Failed(f) => anyhow::bail!("Execute job failed: {f:?}"),
        TerminalStatus::Cancelled => anyhow::bail!("Execute job was cancelled."),
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
            let (hash_id, _) = register_elf(&client, elf)?;
            println!("Register completed. hash_id: {hash_id}");
        }

        Commands::Setup { elf, with_hints, emulator_only } => {
            let (hash_id, program_name) = register_elf(&client, elf)?;
            run_setup(&client, &hash_id, program_name, *with_hints, *emulator_only)?;
            println!("Setup completed for hash_id: {hash_id}");
        }

        Commands::Prove { hash_id, input, hints, proof, output, timeout } => {
            run_prove(
                &client,
                hash_id,
                input.as_ref(),
                hints.as_ref(),
                proof,
                output.as_ref(),
                *timeout,
            )?;
        }

        Commands::Execute { hash_id, input, hints, output, timeout } => {
            run_execute(
                &client,
                hash_id,
                input.as_ref(),
                hints.as_ref(),
                output.as_ref(),
                *timeout,
            )?;
        }

        Commands::Cancel { job_id } => {
            info!("Cancelling job {job_id} …");
            let cancelled = client.cancel_job(*job_id)?;
            if cancelled {
                println!("Job {job_id} cancelled.");
            } else {
                println!("Job {job_id} was not cancelled (already in a terminal state).");
            }
        }
    }

    Ok(())
}
