use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::Result;
use cargo_zisk::commands::{get_proving_key, get_witness_computation_lib};
use distributed_common::{ComputeCapacity, ProverId};
use distributed_prover::{
    config::{ConnectionConfig, ProverClientConfig},
    ProverServiceConfig,
};
use proofman_common::{json_to_debug_instances_map, DebugInfo, ParamsGPU};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use serde::{Deserialize, Serialize};

/// Client configuration structure that can be loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverGrpcEndpointConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Prover configuration
    pub prover: ProverConfig,

    /// Connection configuration
    #[serde(default)]
    pub connection: ConnectionConfig,
}

impl ProverGrpcEndpointConfig {
    /// Load configuration from a specific file
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ProverGrpcEndpointConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Apply CLI overrides to the configuration
    pub fn apply_cli_overrides(
        &mut self,
        url: Option<String>,
        prover_id: Option<String>,
        compute_units: Option<u32>,
        num_nodes: Option<u32>,
    ) {
        if let Some(url) = url {
            self.server.url = url;
        }

        if let Some(prover_id) = prover_id {
            self.prover.prover_id = ProverId::from(prover_id);
        }

        if let Some(compute_units) = compute_units {
            self.prover.compute_capacity.compute_units = compute_units;
        }

        if let Some(num_nodes) = num_nodes {
            self.prover.num_nodes = num_nodes;
        }
    }

    /// Get the prover ID, generating one if not set
    pub fn get_prover_id(&self) -> ProverId {
        self.prover.prover_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server URL to connect to
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverConfig {
    /// Prover ID (optional, will auto-generate if not provided)
    pub prover_id: ProverId,

    /// Compute capacity configuration
    pub compute_capacity: ComputeCapacity,

    pub num_nodes: u32,
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
    num_nodes: Option<u32>,
) -> Result<(ProverGrpcEndpointConfig, ProverServiceConfig)> {
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

    //TODO! CHECK THIS
    let shared_tables = false;

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
        prover_config.asm_port,
        prover_config.unlock_mapped_memory,
        prover_config.verify_constraints,
        prover_config.aggregation,
        prover_config.final_snark,
        gpu_params,
        shared_tables,
    );

    // Load gRPC configuration
    let mut grpc_config = if std::path::Path::new(grpc_config_path).exists() {
        ProverGrpcEndpointConfig::load_from_file(grpc_config_path)?
    } else {
        return Err(anyhow::anyhow!("Configuration file '{}' not found.", grpc_config_path));
    };

    // Apply CLI overrides if provided
    grpc_config.apply_cli_overrides(url, prover_id, compute_units, num_nodes);

    // Validate required fields
    if grpc_config.server.url.is_empty() {
        return Err(anyhow::anyhow!("Server URL is required. Set it in config file or use --url"));
    }

    Ok((grpc_config, service_config))
}
