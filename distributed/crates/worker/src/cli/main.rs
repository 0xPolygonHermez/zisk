use anyhow::Result;
use cargo_zisk::commands::{get_proving_key, get_witness_computation_lib};
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use zisk_distributed_worker::{
    config::{build_worker_and_prover_config, ProverServiceConfigDto},
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
    compute_units: Option<u32>,

    #[clap(
        short = 'j',
        long,
        default_value_t = false,
        help = "Whether to share tables when worker is running in a cluster"
    )]
    pub shared_tables: bool,

    /// Path to configuration file
    #[arg(long, default_value = "config.toml")]
    config: String,

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

    #[clap(short = 'c', long)]
    pub chunk_size_bits: Option<u64>,

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
    // Initialize tracing
    zisk_distributed_common::tracing::init()?;

    let cli = Cli::parse();

    let prover_config = ProverServiceConfigDto {
        elf: cli.elf.clone(),
        witness_lib: cli.witness_lib.clone(),
        asm: cli.asm.clone(),
        chunk_size_bits: cli.chunk_size_bits,
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

    let (grpc_config, service_config) = build_worker_and_prover_config(
        prover_config,
        &cli.config,
        cli.coordinator_url,
        cli.worker_id,
        cli.compute_units,
    )
    .await?;

    print_command_info(&service_config, cli.debug.is_some());

    let mut worker = WorkerNode::new(grpc_config, service_config).await?;
    worker.run().await
}

fn print_command_info(service_config: &ProverConfig, debug: bool) {
    println!("{} ZisK Worker", format!("{: >12}", "Command").bright_green().bold());
    println!(
        "{: >12} {}",
        "Witness Lib".bright_green().bold(),
        get_witness_computation_lib(Some(&service_config.witness_lib)).display()
    );

    println!("{: >12} {}", "Elf".bright_green().bold(), service_config.elf.display());

    if service_config.asm.is_some() {
        let asm_path = service_config.asm.as_ref().unwrap().display();
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
        get_proving_key(Some(&service_config.proving_key)).display()
    );

    let std_mode = if debug { "Debug mode" } else { "Standard mode" };
    println!("{: >12} {}", "STD".bright_green().bold(), std_mode);

    println!();
}
