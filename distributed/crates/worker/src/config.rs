use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zisk_cluster_common::Environment;
use zisk_cluster_common::{ComputeCapacity, LoggingConfig, WorkerId};

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

/// Worker identity and capacity settings.
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

/// Which coordinator the worker connects to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Coordinator URL to connect to
    pub url: String,
}

/// Connection resilience settings (reconnect interval, heartbeat timeout).
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
    /// Default reconnection interval, in seconds (serde default).
    pub const fn default_reconnect_interval() -> u64 {
        Self::DEFAULT_RECONNECT_INTERVAL
    }

    /// Default heartbeat timeout, in seconds (serde default).
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
    /// Load the worker configuration from built-in defaults, an optional TOML
    /// file (or `ZISK_WORKER_CONFIG_PATH`), and the given argument overrides.
    /// A random worker id is generated when none is provided.
    pub async fn load(
        config: Option<String>,
        coordinator_url: Option<String>,
        worker_id: Option<String>,
        compute_capacity: Option<u32>,
    ) -> Result<Self> {
        // Config file is now optional - if not provided, defaults will be used
        let config = config.or_else(|| std::env::var("ZISK_WORKER_CONFIG_PATH").ok());

        // Generate a random worker ID
        let random_worker_id = format!("{}", uuid::Uuid::new_v4().simple());

        let mut builder = config::Config::builder()
            .set_default("worker.worker_id", random_worker_id)?
            .set_default("worker.compute_capacity.compute_units", 10)?
            .set_default("worker.environment", "development")?
            .set_default("worker.inputs_folder", ".")?
            .set_default("coordinator.url", zisk_coordinator::Config::default_url())?
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

        let config = builder.build()?;

        Ok(config.try_deserialize()?)
    }
}

/// Configuration for initializing a Prover Service
#[derive(Debug, Default, Clone)]
pub struct ProverServiceConfigDto {
    /// Path to prebuilt ASM binaries; `None` uses the default cache.
    pub asm: Option<PathBuf>,
    /// Use the pure-Rust emulator backend instead of ASM.
    pub emulator: bool,
    /// Path to the proving key; `None` uses the default.
    pub proving_key: Option<PathBuf>,
    /// Path to the SNARK proving key; `None` uses the default.
    pub proving_key_snark: Option<PathBuf>,
    /// Unlock mapped memory for the ASM shared-memory regions.
    pub unlock_mapped_memory: bool,
    /// Write the ASM emulator output to a file instead of shared memory.
    pub asm_out_file: bool,
    /// Verbosity level (`0` = quiet).
    pub verbose: u8,
    /// Constraint-debug selector (see the prover backend's debug info).
    pub debug: Option<Option<String>>,
    /// Cap on concurrent proving streams.
    pub max_streams: Option<usize>,
    /// Cap on concurrent recursive (aggregation) streams.
    pub max_recursive_streams: Option<usize>,
    /// Size of the witness-generation thread pool.
    pub number_threads_witness: Option<usize>,
    /// Cap on witnesses kept in memory at once.
    pub max_witness_stored: Option<usize>,
    /// Prefer lower memory usage over speed.
    pub minimal_memory: bool,
    /// Use the GPU proving path.
    pub gpu: bool,
    /// Enable PLONK/SNARK proof generation.
    pub plonk: bool,
    /// Preload the PLONK/SNARK proving keys.
    pub preload_plonk: bool,
    /// Use the CPU MOPs
    pub cpu_mops: bool,
}
