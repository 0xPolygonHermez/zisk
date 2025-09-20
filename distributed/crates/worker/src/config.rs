use crate::ProverServiceConfig;

use anyhow::Result;
use cargo_zisk::commands::{get_proving_key, get_witness_computation_lib};
use proofman_common::{json_to_debug_instances_map, DebugInfo, ParamsGPU};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{collections::HashMap, fs};
use zisk_distributed_common::{ComputeCapacity, WorkerId};

/// Worker Service Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerServiceConfig {
    /// Worker configuration
    pub worker: WorkerConfig,

    /// Coordinator configuration
    pub coordinator: CoordinatorConfig,

    /// Connection configuration
    #[serde(default)]
    pub connection: ConnectionConfig,
}

impl WorkerServiceConfig {
    /// Load configuration from a specific file
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: WorkerServiceConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Apply CLI overrides to the configuration
    pub fn apply_cli_overrides(
        &mut self,
        url: Option<String>,
        worker_id: Option<String>,
        compute_units: Option<u32>,
    ) {
        if let Some(url) = url {
            self.coordinator.url = url;
        }

        if let Some(worker_id) = worker_id {
            self.worker.worker_id = WorkerId::from(worker_id);
        }

        if let Some(compute_units) = compute_units {
            self.worker.compute_capacity.compute_units = compute_units;
        }
    }

    pub fn worker_id(&self) -> WorkerId {
        self.worker.worker_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Worker ID (optional, will auto-generate if not provided)
    pub worker_id: WorkerId,

    /// Compute capacity configuration
    pub compute_capacity: ComputeCapacity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Coordinator URL to connect to
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Reconnection interval in seconds
    #[serde(default = "ConnectionConfig::default_reconnect_interval")]
    pub reconnect_interval_seconds: u64,

    /// Heartbeat timeout in seconds
    #[serde(default = "ConnectionConfig::default_heartbeat_timeout")]
    pub heartbeat_timeout_seconds: u64,
}

impl ConnectionConfig {
    const DEFAULT_RECONNECT_INTERVAL: u64 = 5;
    const DEFAULT_HEARTBEAT_TIMEOUT: u64 = 30;

    // These are needed for serde's `default` attribute
    pub const fn default_reconnect_interval() -> u64 {
        Self::DEFAULT_RECONNECT_INTERVAL
    }

    pub const fn default_heartbeat_timeout() -> u64 {
        Self::DEFAULT_HEARTBEAT_TIMEOUT
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            reconnect_interval_seconds: Self::DEFAULT_RECONNECT_INTERVAL,
            heartbeat_timeout_seconds: Self::DEFAULT_HEARTBEAT_TIMEOUT,
        }
    }
}

/// Configuration for initializing a Prover Service
#[derive(Debug, Clone)]
pub struct ProverServiceConfigDto {
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

impl Default for ProverServiceConfigDto {
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

/// Builds the [`WorkerServiceConfig`] and [`ProverServiceConfig`] from the given inputs.
pub async fn build_worker_and_prover_config(
    mut prover_service_config: ProverServiceConfigDto,
    config_path: &str,
    url: Option<String>,
    worker_id: Option<String>,
    compute_units: Option<u32>,
) -> Result<(WorkerServiceConfig, ProverServiceConfig)> {
    // Validate ELF file
    if !prover_service_config.elf.exists() {
        return Err(anyhow::anyhow!(
            "ELF file '{}' not found.",
            prover_service_config.elf.display()
        ));
    }

    let proving_key = get_proving_key(prover_service_config.proving_key.as_ref());

    let debug_info = match &prover_service_config.debug {
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

    let emulator = if cfg!(target_os = "macos") { true } else { prover_service_config.emulator };

    let mut asm_rom = None;
    if emulator {
        prover_service_config.asm = None;
    } else if prover_service_config.asm.is_none() {
        let stem = prover_service_config.elf.file_stem().unwrap().to_str().unwrap();
        let hash = get_elf_data_hash(&prover_service_config.elf)
            .map_err(|e| anyhow::anyhow!("Error computing ELF hash: {}", e))?;
        let new_filename = format!("{stem}-{hash}-mt.bin");
        let asm_rom_filename = format!("{stem}-{hash}-rh.bin");
        asm_rom = Some(default_cache_path.join(asm_rom_filename));
        prover_service_config.asm = Some(default_cache_path.join(new_filename));
    }

    if let Some(asm_path) = &prover_service_config.asm {
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
        &prover_service_config.elf.to_path_buf(),
        &default_cache_path,
        blowup_factor,
    )?;

    if !rom_bin_path.exists() {
        let _ = gen_elf_hash(
            &prover_service_config.elf.clone(),
            rom_bin_path.as_path(),
            blowup_factor,
            false,
        )
        .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
    }

    let mut custom_commits_map: HashMap<String, PathBuf> = HashMap::new();
    custom_commits_map.insert("rom".to_string(), rom_bin_path);

    let mut gpu_params = ParamsGPU::new(prover_service_config.preallocate);

    if prover_service_config.max_streams.is_some() {
        gpu_params.with_max_number_streams(prover_service_config.max_streams.unwrap());
    }
    if prover_service_config.number_threads_witness.is_some() {
        gpu_params.with_number_threads_pools_witness(
            prover_service_config.number_threads_witness.unwrap(),
        );
    }
    if prover_service_config.max_witness_stored.is_some() {
        gpu_params.with_max_witness_stored(prover_service_config.max_witness_stored.unwrap());
    }

    //TODO! CHECK THIS
    let shared_tables = false;

    let service_config = ProverServiceConfig::new(
        prover_service_config.elf.clone(),
        get_witness_computation_lib(prover_service_config.witness_lib.as_ref()),
        prover_service_config.asm.clone(),
        asm_rom,
        custom_commits_map,
        emulator,
        proving_key,
        prover_service_config.verbose,
        debug_info,
        prover_service_config.chunk_size_bits,
        prover_service_config.asm_port,
        prover_service_config.unlock_mapped_memory,
        prover_service_config.verify_constraints,
        prover_service_config.aggregation,
        prover_service_config.final_snark,
        gpu_params,
        shared_tables,
    );

    // Load gRPC configuration
    let mut grpc_config = if std::path::Path::new(config_path).exists() {
        WorkerServiceConfig::load_from_file(config_path)?
    } else {
        return Err(anyhow::anyhow!("Configuration file '{}' not found.", config_path));
    };

    // Apply CLI overrides if provided
    grpc_config.apply_cli_overrides(url, worker_id, compute_units);

    // Validate required fields
    if grpc_config.coordinator.url.is_empty() {
        return Err(anyhow::anyhow!(
            "Coordinator URL is required. Set it in config file or use --url"
        ));
    }

    Ok((grpc_config, service_config))
}
