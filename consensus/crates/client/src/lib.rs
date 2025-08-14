//! # Consensus Client Library
//!
//! This library provides the core functionality for a consensus network prover client.
//! It includes configuration management, proof generation, gRPC communication,
//! and prover service management.
pub mod config;
pub mod proof_generator;
pub mod prover_grpc_endpoint;
pub mod prover_service;

pub use config::ProverGrpcEndpointConfig;
pub use proof_generator::ProofGenerator;
pub use prover_grpc_endpoint::{ComputationResult, ProverGrpcEndpoint};
pub use prover_service::{JobContext, ProverService, ProverServiceConfig};

use anyhow::Result;
use asm_runner::AsmRunnerOptions;
use cargo_zisk::commands::{get_proving_key, get_witness_computation_lib, initialize_mpi};
use proofman_common::{json_to_debug_instances_map, DebugInfo, ParamsGPU};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use std::{collections::HashMap, fs, path::PathBuf};
use zisk_common::MpiContext;

/// Configuration for initializing a prover client
#[derive(Debug, Clone)]
pub struct ProverClientConfig {
    pub elf: PathBuf,
    pub witness_lib: Option<PathBuf>,
    pub asm: Option<PathBuf>,
    pub chunk_size_bits: Option<u64>,
    pub emulator: bool,
    pub proving_key: Option<PathBuf>,
    pub asm_port: Option<u16>,
    pub unlock_mapped_memory: bool,
    pub verbose: u8,
    pub debug: Option<Option<String>>,
    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,
    pub preallocate: bool,
    pub max_streams: Option<usize>,
    pub number_threads_witness: Option<usize>,
    pub max_witness_stored: Option<usize>,
}

impl Default for ProverClientConfig {
    fn default() -> Self {
        Self {
            elf: PathBuf::new(),
            witness_lib: None,
            asm: None,
            chunk_size_bits: None,
            emulator: false,
            proving_key: None,
            asm_port: None,
            unlock_mapped_memory: false,
            verbose: 0,
            debug: None,
            verify_constraints: false,
            aggregation: false,
            final_snark: false,
            preallocate: false,
            max_streams: None,
            number_threads_witness: None,
            max_witness_stored: None,
        }
    }
}

/// Initialize and configure a prover client with the given configuration
///
/// Returns a configured `ProverGrpcEndpoint` ready to run.
pub async fn initialize_prover_config(
    mut prover_config: ProverClientConfig,
    grpc_config_path: &str,
    url: Option<String>,
    prover_id: Option<String>,
    compute_units: Option<u32>,
) -> Result<(ProverGrpcEndpointConfig, ProverServiceConfig, MpiContext)> {
    let mpi_context = initialize_mpi()?;

    proofman_common::initialize_logger(
        proofman_common::VerboseMode::Info,
        Some(mpi_context.world_rank),
    );

    // Validate ELF file
    if !prover_config.elf.exists() {
        return Err(anyhow::anyhow!("ELF file '{}' not found.", prover_config.elf.display()));
    }

    let proving_key = get_proving_key(prover_config.proving_key.as_ref());

    let debug_info = match &prover_config.debug {
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
                return Err(anyhow::anyhow!("Failed to create the cache directory: {e:?}"));
            }
        }
    }

    let emulator = if cfg!(target_os = "macos") { true } else { prover_config.emulator };

    let mut asm_rom = None;
    if emulator {
        prover_config.asm = None;
    } else if prover_config.asm.is_none() {
        let stem = prover_config.elf.file_stem().unwrap().to_str().unwrap();
        let hash = get_elf_data_hash(&prover_config.elf)
            .map_err(|e| anyhow::anyhow!("Error computing ELF hash: {}", e))?;
        let new_filename = format!("{stem}-{hash}-mt.bin");
        let asm_rom_filename = format!("{stem}-{hash}-rh.bin");
        asm_rom = Some(default_cache_path.join(asm_rom_filename));
        prover_config.asm = Some(default_cache_path.join(new_filename));
    }

    if let Some(asm_path) = &prover_config.asm {
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

    let rom_bin_path = get_elf_bin_file_path(
        &prover_config.elf.to_path_buf(),
        &default_cache_path,
        blowup_factor,
    )?;

    if !rom_bin_path.exists() {
        let _ =
            gen_elf_hash(&prover_config.elf.clone(), rom_bin_path.as_path(), blowup_factor, false)
                .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
    }

    let mut custom_commits_map: HashMap<String, PathBuf> = HashMap::new();
    custom_commits_map.insert("rom".to_string(), rom_bin_path);

    let asm_runner_options = AsmRunnerOptions::new()
        .with_verbose(prover_config.verbose > 0)
        .with_base_port(prover_config.asm_port)
        .with_world_rank(mpi_context.world_rank)
        .with_local_rank(mpi_context.local_rank)
        .with_unlock_mapped_memory(prover_config.unlock_mapped_memory);

    let mut gpu_params = ParamsGPU::new(prover_config.preallocate);

    if prover_config.max_streams.is_some() {
        gpu_params.with_max_number_streams(prover_config.max_streams.unwrap());
    }
    if prover_config.number_threads_witness.is_some() {
        gpu_params.with_number_threads_pools_witness(prover_config.number_threads_witness.unwrap());
    }
    if prover_config.max_witness_stored.is_some() {
        gpu_params.with_max_witness_stored(prover_config.max_witness_stored.unwrap());
    }

    let service_config = ProverServiceConfig::new(
        prover_config.elf.clone(),
        get_witness_computation_lib(prover_config.witness_lib.as_ref()),
        prover_config.asm.clone(),
        asm_rom,
        custom_commits_map,
        emulator,
        proving_key,
        prover_config.verbose,
        debug_info,
        prover_config.chunk_size_bits,
        asm_runner_options,
        prover_config.verify_constraints,
        prover_config.aggregation,
        prover_config.final_snark,
        gpu_params,
    );

    // Load gRPC configuration
    let mut grpc_config = if std::path::Path::new(grpc_config_path).exists() {
        ProverGrpcEndpointConfig::load_from_file(grpc_config_path)?
    } else {
        return Err(anyhow::anyhow!("Configuration file '{}' not found.", grpc_config_path));
    };

    // Apply CLI overrides if provided
    grpc_config.apply_cli_overrides(url, prover_id, compute_units);

    // Validate required fields
    if grpc_config.server.url.is_empty() {
        return Err(anyhow::anyhow!("Server URL is required. Set it in config file or use --url"));
    }

    Ok((grpc_config, service_config, mpi_context))
}

pub async fn build_prover_endpoint(
    grpc_config: ProverGrpcEndpointConfig,
    service_config: ProverServiceConfig,
    mpi_context: MpiContext,
) -> Result<ProverGrpcEndpoint> {
    ProverGrpcEndpoint::new(grpc_config, service_config, mpi_context)
}
