use anyhow::Result;
use cargo_zisk::{
    commands::{get_proving_key, get_witness_computation_lib},
    ux::print_banner,
};
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use zisk_distributed_worker::{
    config::{ProverServiceConfigDto, WorkerServiceConfig},
    ProverConfig, WorkerNode,
};

#[derive(Parser)]
#[command(name = "zisk-worker")]
#[command(about = "A Worker for the Distributed ZisK Network")]
#[command(version)]
struct Cli {
    /// Distributed ZisK Coordinator URL (overrides config file)
    #[arg(short, long)]
    coordinator_url: Option<String>,

    /// Worker ID (overrides config file, defaults to auto-generated UUID)
    #[arg(long)]
    worker_id: Option<String>,

    /// Number of compute units to advertise (overrides config file)
    #[arg(long)]
    compute_capacity: Option<u32>,

    /// This is the path where the worker will look for input files to process.
    #[clap(short = 'i', long)]
    inputs_folder: Option<PathBuf>,

    #[clap(
        short = 'j',
        long,
        default_value_t = false,
        help = "Whether to share tables when worker is running in a cluster"
    )]
    pub shared_tables: bool,

    /// Path to configuration file
    #[arg(
        long,
        help = "Path to configuration file (overrides ZISK_WORKER_CONFIG_PATH environment variable)"
    )]
    config: Option<String>,

    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ELF file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// ASM file path
    /// Optional, mutually exclusive with `--emulator`
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Base port for Assembly microservices (default: 23115).
    /// A single execution will use 3 consecutive ports, from this port to port + 2.
    /// If you are running multiple instances of ZisK using mpi on the same machine,
    /// it will use from this base port to base port + 2 * number_of_instances.
    /// For example, if you run 2 mpi instances of ZisK, it will use ports from 23115 to 23117
    /// for the first instance, and from 23118 to 23120 for the second instance.
    #[clap(long, conflicts_with = "emulator")]
    pub asm_port: Option<u16>,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    /// If you are running ZisK on a machine with limited memory, you may want to enable this option.
    /// This option is mutually exclusive with `--emulator`.
    #[clap(long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    /// Whether to verify constraints
    #[clap(long, default_value_t = false)]
    pub verify_constraints: bool,

    /// Whether to generate the final SNARK
    #[clap(short = 'f', long, default_value_t = false)]
    pub final_snark: bool,

    /// GPU parameters
    #[clap(short = 'r', long, default_value_t = false)]
    pub preallocate: bool,

    /// Maximum number of GPU streams
    #[clap(short = 't', long)]
    pub max_streams: Option<usize>,

    #[clap(short = 'n', long)]
    pub number_threads_witness: Option<usize>,

    #[clap(short = 'x', long)]
    pub max_witness_stored: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let worker_config = WorkerServiceConfig::load(
        cli.config,
        cli.coordinator_url,
        cli.worker_id,
        cli.compute_capacity,
        cli.inputs_folder,
    )
    .await?;

    // Initialize tracing - keep guard alive for application lifetime
    let _log_guard = zisk_distributed_common::tracing::init(Some(&worker_config.logging))?;

    print_banner();

    let prover_config_dto = ProverServiceConfigDto {
        elf: cli.elf.clone(),
        witness_lib: cli.witness_lib.clone(),
        asm: cli.asm.clone(),
        emulator: cli.emulator,
        proving_key: cli.proving_key.clone(),
        asm_port: cli.asm_port,
        unlock_mapped_memory: cli.unlock_mapped_memory,
        verbose: cli.verbose,
        debug: cli.debug.clone(),
        verify_constraints: cli.verify_constraints,
        aggregation: true, // we always aggregate
        final_snark: cli.final_snark,
        preallocate: cli.preallocate,
        max_streams: cli.max_streams,
        number_threads_witness: cli.number_threads_witness,
        max_witness_stored: cli.max_witness_stored,
        shared_tables: cli.shared_tables,
    };

    let prover_config = ProverConfig::load(prover_config_dto)?;

    print_command_info(&prover_config, &worker_config, cli.debug.is_some());

    let mut worker = WorkerNode::new(worker_config, prover_config).await?;
    worker.run().await
}

fn print_command_info(
    prover_config: &ProverConfig,
    worker_config: &WorkerServiceConfig,
    debug: bool,
) {
    println!(
        "{} zisk-worker (ZisK Distributed Worker {})",
        format!("{: >12}", "Command").bright_green().bold(),
        env!("CARGO_PKG_VERSION")
    );
    println!("{: >12} {}", "Worker ID".bright_green().bold(), worker_config.worker.worker_id);
    println!(
        "{: >12} {}",
        "Compute Cap".bright_green().bold(),
        worker_config.worker.compute_capacity
    );
    println!("{: >12} {}", "Coordinator".bright_green().bold(), worker_config.coordinator.url);
    println!("{: >12} {}", "Environment".bright_green().bold(), worker_config.worker.environment);
    println!(
        "{: >12} {}/{} {}",
        "Logging".bright_green().bold(),
        worker_config.logging.level,
        worker_config.logging.format,
        worker_config
            .logging
            .file_path
            .as_deref()
            .map(|p| format!("(log file: {})", p).bright_black().to_string())
            .unwrap_or_default()
    );
    println!(
        "{: >12} {}",
        "Witness Lib".bright_green().bold(),
        get_witness_computation_lib(Some(&prover_config.witness_lib)).display()
    );

    println!("{: >12} {}", "Elf".bright_green().bold(), prover_config.elf.display());
    if prover_config.asm.is_some() {
        if let Some(asm_port) = prover_config.asm_port.as_ref() {
            println!("{: >12} {}", "Asm port".bright_green().bold(), asm_port);
        }
        let asm_path = prover_config.asm.as_ref().unwrap().display();
        println!("{: >12} {}", "ASM runner".bright_green().bold(), asm_path);
    } else {
        println!(
            "{: >12} {}",
            "Emulator".bright_green().bold(),
            "Running in emulator mode".bright_yellow()
        );
    }
    println!(
        "{: >12} {}",
        "Proving key".bright_green().bold(),
        get_proving_key(Some(&prover_config.proving_key)).display()
    );

    let std_mode = if debug { "Debug mode" } else { "Standard mode" };
    println!("{: >12} {}", "STD".bright_green().bold(), std_mode);

    println!();
}
