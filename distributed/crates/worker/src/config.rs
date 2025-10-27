use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zisk_distributed_common::Environment;
use zisk_distributed_common::{ComputeCapacity, LoggingConfig, WorkerId};

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

    /// Logging configuration
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Worker ID (optional, will auto-generate if not provided)
    pub worker_id: WorkerId,

    /// Compute capacity configuration
    pub compute_capacity: ComputeCapacity,

    /// Environment (e.g., development, production)
    pub environment: Environment,

    /// This is the path where the worker will look for input files to process. By default, it is the current directory.
    pub inputs_folder: PathBuf,
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

impl WorkerServiceConfig {
    pub async fn load(
        config: Option<String>,
        coordinator_url: Option<String>,
        worker_id: Option<String>,
        compute_capacity: Option<u32>,
        inputs_folder: Option<PathBuf>,
    ) -> Result<Self> {
        // Config file is now optional - if not provided, defaults will be used
        let config = config.or_else(|| std::env::var("ZISK_WORKER_CONFIG_PATH").ok());

        // Check inputs folder exists if provided
        if let Some(ref path) = inputs_folder {
            if !path.exists() || !path.is_dir() {
                anyhow::bail!(
                    "Inputs folder does not exist or is not a directory: {}",
                    path.display()
                );
            }
        }

        // Generate a random worker ID
        let random_worker_id = format!("{}", uuid::Uuid::new_v4().simple());

        let mut builder = config::Config::builder()
            .set_default("worker.worker_id", random_worker_id)?
            .set_default("worker.compute_capacity.compute_units", 10)?
            .set_default("worker.environment", "development")?
            .set_default("worker.inputs_folder", ".")?
            .set_default("coordinator.url", zisk_distributed_coordinator::Config::default_url())?
            .set_default("connection.reconnect_interval_seconds", 5)?
            .set_default("connection.heartbeat_timeout_seconds", 30)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?;

        if let Some(path) = config {
            builder = builder.add_source(config::File::with_name(&path));
        }

        if let Some(coordinator_url) = coordinator_url {
            builder = builder.set_override("coordinator.url", coordinator_url)?;
        }

        if let Some(worker_id) = worker_id {
            builder = builder.set_override("worker.worker_id", worker_id)?;
        }

        if let Some(compute_capacity) = compute_capacity {
            builder =
                builder.set_override("worker.compute_capacity.compute_units", compute_capacity)?;
        }

        if let Some(inputs_folder) = inputs_folder {
            builder = builder.set_override(
                "worker.inputs_folder",
                inputs_folder.to_string_lossy().to_string(),
            )?;
        }

        let config = builder.build()?;

        Ok(config.try_deserialize()?)
    }
}

/// Configuration for initializing a Prover Service
#[derive(Debug, Clone)]
pub struct ProverServiceConfigDto {
    pub elf: PathBuf,
    pub witness_lib: Option<PathBuf>,
    pub asm: Option<PathBuf>,
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
    pub shared_tables: bool,
}

impl Default for ProverServiceConfigDto {
    fn default() -> Self {
        Self {
            elf: PathBuf::new(),
            witness_lib: None,
            asm: None,
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
            shared_tables: false,
        }
    }
}
