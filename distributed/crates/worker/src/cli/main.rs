use anyhow::Result;
use cargo_zisk::ux::print_banner;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use zisk_prover_backend::{Asm, Emu};
use zisk_worker::{
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

    /// Path to configuration file
    #[arg(
        long,
        help = "Path to configuration file (overrides ZISK_WORKER_CONFIG_PATH environment variable)"
    )]
    config: Option<String>,

    /// ASM file path
    /// Optional, mutually exclusive with `--emulator`
    #[clap(short = 'a', long)]
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Setup folder path
    #[clap(short = 's', long)]
    pub proving_key_snark: Option<PathBuf>,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    /// If you are running ZisK on a machine with limited memory, you may want to enable this option.
    /// This option is mutually exclusive with `--emulator`.
    #[clap(long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Redirect ASM emulator output to file
    /// This option is mutually exclusive with `--emulator`
    #[clap(long, conflicts_with = "emulator", default_value_t = false)]
    pub asm_out_file: bool,

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    /// Whether to verify constraints
    #[clap(long, default_value_t = false)]
    pub verify_constraints: bool,

    /// Maximum number of GPU streams
    #[clap(short = 't', long)]
    pub max_streams: Option<usize>,

    #[clap(short = 'n', long)]
    pub number_threads_witness: Option<usize>,

    #[clap(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    #[clap(short = 'm', long, default_value_t = false)]
    pub minimal_memory: bool,

    #[clap(long, default_value_t = false)]
    pub hints: bool,

    #[cfg(not(feature = "cpu-only"))]
    #[clap(short = 'g', long, default_value_t = false)]
    pub gpu: bool,

    #[clap(short = 'p', long, default_value_t = false)]
    pub plonk: bool,

    #[clap(short = 'P', long, default_value_t = false)]
    pub preload_plonk: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let worker_config = WorkerServiceConfig::load(
        cli.config,
        cli.coordinator_url,
        cli.worker_id,
        cli.compute_capacity,
    )
    .await?;

    print_banner();

    #[cfg(not(feature = "cpu-only"))]
    let gpu = cli.gpu;
    #[cfg(feature = "cpu-only")]
    let gpu = false;

    let prover_config_dto = ProverServiceConfigDto {
        asm: cli.asm.clone(),
        emulator: cli.emulator,
        hints: cli.hints,
        proving_key: cli.proving_key.clone(),
        proving_key_snark: cli.proving_key_snark.clone(),
        unlock_mapped_memory: cli.unlock_mapped_memory,
        asm_out_file: cli.asm_out_file,
        verbose: cli.verbose,
        debug: cli.debug.clone(),
        max_streams: cli.max_streams,
        number_threads_witness: cli.number_threads_witness,
        max_witness_stored: cli.max_witness_stored,
        minimal_memory: cli.minimal_memory,
        preload_plonk: cli.preload_plonk,
        gpu,
        plonk: cli.plonk,
    };

    let prover_config = ProverConfig::load(prover_config_dto)?;

    print_command_info(&prover_config, &worker_config, cli.debug.is_some());

    if prover_config.emulator {
        let mut worker = WorkerNode::<Emu>::new_emu(worker_config, prover_config).await?;
        return worker.run().await;
    } else {
        let mut worker = WorkerNode::<Asm>::new_asm(worker_config, prover_config).await?;
        return worker.run().await;
    };
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

    if prover_config.emulator {
        println!(
            "{: >12} {}",
            "Emulator".bright_green().bold(),
            "Running in emulator mode".bright_yellow()
        );
    }
    println!(
        "{: >12} {}",
        "Proving Key".bright_green().bold(),
        prover_config.proving_key.display()
    );

    let std_mode = if debug { "Debug mode" } else { "Standard mode" };
    println!("{: >12} {}", "STD".bright_green().bold(), std_mode);

    println!();
}
