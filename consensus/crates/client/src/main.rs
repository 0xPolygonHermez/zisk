mod config;
mod prover_grpc_endpoint;
mod prover_service;

use anyhow::Result;
use asm_runner::AsmRunnerOptions;
use cargo_zisk::commands::{get_proving_key, get_witness_computation_lib, initialize_mpi};
use clap::Parser;
use colored::Colorize;
use proofman_common::{json_to_debug_instances_map, DebugInfo, ParamsGPU};
use prover_grpc_endpoint::ProverGrpcEndpoint;
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use std::{collections::HashMap, fs, path::PathBuf, process};
use tracing::info;

use crate::{config::ProverGrpcEndpointConfig, prover_service::ProverServiceConfig};

#[derive(Parser)]
#[command(name = "consensus-client")]
#[command(about = "A prover client for the Consensus Network")]
#[command(version)]
struct Cli {
    /// Server URL (overrides config file)
    #[arg(short, long)]
    url: Option<String>,

    /// Prover ID (overrides config file, defaults to auto-generated UUID)
    #[arg(long)]
    prover_id: Option<String>,

    /// Number of compute units to advertise (overrides config file)
    #[arg(long)]
    compute_units: Option<u32>,

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

    #[clap(long, default_value_t = false)]
    pub verify_constraints: bool,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'f', long, default_value_t = false)]
    pub final_snark: bool,

    /// GPU PARAMS
    #[clap(short = 'r', long, default_value_t = false)]
    pub preallocate: bool,

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
    consensus_core::tracing::init()?;

    let mut cli = Cli::parse();

    let mpi_context = initialize_mpi()?;

    proofman_common::initialize_logger(
        proofman_common::VerboseMode::Info,
        Some(mpi_context.world_rank),
    );

    if !cli.elf.exists() {
        eprintln!("Error: ELF file '{}' not found.", cli.elf.display());
        process::exit(1);
    }

    let proving_key = get_proving_key(cli.proving_key.as_ref());

    let debug_info = match &cli.debug {
        None => DebugInfo::default(),
        Some(None) => DebugInfo::new_debug(),
        Some(Some(debug_value)) => {
            json_to_debug_instances_map(proving_key.clone(), debug_value.clone())
        }
    };

    let default_cache_path =
        std::env::var("HOME").ok().map(PathBuf::from).unwrap().join(DEFAULT_CACHE_PATH);

    if !default_cache_path.exists() {
        if let Err(e) = fs::create_dir_all(default_cache_path.clone()) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                // prevent collision in distributed mode
                panic!("Failed to create the cache directory: {e:?}");
            }
        }
    }

    let emulator = if cfg!(target_os = "macos") { true } else { cli.emulator };

    let mut asm_rom = None;
    if emulator {
        cli.asm = None;
    } else if cli.asm.is_none() {
        let stem = cli.elf.file_stem().unwrap().to_str().unwrap();
        let hash = get_elf_data_hash(&cli.elf)
            .map_err(|e| anyhow::anyhow!("Error computing ELF hash: {}", e))?;
        let new_filename = format!("{stem}-{hash}-mt.bin");
        let asm_rom_filename = format!("{stem}-{hash}-rh.bin");
        asm_rom = Some(default_cache_path.join(asm_rom_filename));
        cli.asm = Some(default_cache_path.join(new_filename));
    }

    if let Some(asm_path) = &cli.asm {
        if !asm_path.exists() {
            return Err(anyhow::anyhow!("ASM file not found at {:?}", asm_path.display()));
        }
    }

    if let Some(asm_rom) = &asm_rom {
        if !asm_rom.exists() {
            return Err(anyhow::anyhow!("ASM file not found at {:?}", asm_rom.display()));
        }
    }

    let blowup_factor = get_rom_blowup_factor(&proving_key);

    let rom_bin_path =
        get_elf_bin_file_path(&cli.elf.to_path_buf(), &default_cache_path, blowup_factor)?;

    if !rom_bin_path.exists() {
        let _ = gen_elf_hash(&cli.elf.clone(), rom_bin_path.as_path(), blowup_factor, false)
            .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
    }

    print_command_info(&cli);

    let mut custom_commits_map: HashMap<String, PathBuf> = HashMap::new();
    custom_commits_map.insert("rom".to_string(), rom_bin_path);

    let asm_runner_options = AsmRunnerOptions::new()
        .with_verbose(cli.verbose > 0)
        .with_base_port(cli.asm_port)
        .with_world_rank(mpi_context.world_rank)
        .with_local_rank(mpi_context.local_rank)
        .with_unlock_mapped_memory(cli.unlock_mapped_memory);

    let mut gpu_params = ParamsGPU::new(cli.preallocate);

    if cli.max_streams.is_some() {
        gpu_params.with_max_number_streams(cli.max_streams.unwrap());
    }
    if cli.number_threads_witness.is_some() {
        gpu_params.with_number_threads_pools_witness(cli.number_threads_witness.unwrap());
    }
    if cli.max_witness_stored.is_some() {
        gpu_params.with_max_witness_stored(cli.max_witness_stored.unwrap());
    }

    let config_xxx = ProverServiceConfig::new(
        cli.elf.clone(),
        get_witness_computation_lib(cli.witness_lib.as_ref()),
        cli.asm.clone(),
        asm_rom,
        custom_commits_map,
        emulator,
        proving_key,
        cli.verbose,
        debug_info,
        cli.chunk_size_bits,
        asm_runner_options,
        cli.verify_constraints,
        cli.aggregation,
        cli.final_snark,
        gpu_params,
    );

    info!("Starting prover client");

    // Load configuration from file
    let mut config = if std::path::Path::new(&cli.config).exists() {
        ProverGrpcEndpointConfig::load_from_file(&cli.config)?
    } else {
        info!("Configuration file '{}' not found, using defaults", cli.config);
        return Err(anyhow::anyhow!("Configuration file '{}' not found.", cli.config));
    };

    // Apply CLI overrides
    config.apply_cli_overrides(cli.url, cli.prover_id, cli.compute_units);

    // Validate required fields
    if config.server.url.is_empty() {
        return Err(anyhow::anyhow!("Server URL is required. Set it in config file or use --url"));
    }

    let prover_id = config.get_prover_id();

    info!(
        "Starting prover client {} ({}) connecting to server {}",
        prover_id, config.prover.compute_capacity, config.server.url
    );

    let mut prover = ProverGrpcEndpoint::new(config, config_xxx, mpi_context)?;

    prover.run().await
}

fn print_command_info(cli: &Cli) {
    println!("{} Prove Client", format!("{: >12}", "Command").bright_green().bold());
    println!(
        "{: >12} {}",
        "Witness Lib".bright_green().bold(),
        get_witness_computation_lib(cli.witness_lib.as_ref()).display()
    );

    println!("{: >12} {}", "Elf".bright_green().bold(), cli.elf.display());

    if cli.asm.is_some() {
        let asm_path = cli.asm.as_ref().unwrap().display();
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
        get_proving_key(cli.proving_key.as_ref()).display()
    );

    let std_mode = if cli.debug.is_some() { "Debug mode" } else { "Standard mode" };
    println!("{: >12} {}", "STD".bright_green().bold(), std_mode);
    // println!("{}", format!("{: >12} {}", "Distributed".bright_green().bold(), "ON (nodes: 4, threads: 32)"));

    println!();
}
