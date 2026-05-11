use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use zisk_cluster_common::Environment;
use zisk_cluster_common::LoggingConfig;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub service: ServiceConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub coordinator: CoordinatorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub proofs_dir: PathBuf,
    pub no_save_proofs: bool,
    pub shutdown_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub version: String,
    pub environment: Environment,
}

/// Coordinator-level knobs for job orchestration, fault tolerance, and worker management.
///
/// ## Timeout mapping
///
/// The job monitor checks running jobs against per-phase timeouts. A phase that
/// exceeds its timeout triggers an immediate `fail_job` (all workers cancelled).
///
/// | Job phase                              | Config field                    | Default |
/// |----------------------------------------|---------------------------------|---------|
/// | `Execution`                            | `execution_timeout_seconds`     | 300 s   |
/// | `Contributions` / `ContributionsInputsStream` / `ContributionsHintsStream` | `phase1_timeout_seconds` | 300 s |
/// | `Prove`                                | `phase2_timeout_seconds`        | 600 s   |
/// | `Aggregate`                            | `phase3_timeout_seconds`        | 100 s   |
///
/// Setting any timeout to `0` disables enforcement for that phase.
///
/// ## Heartbeat detection
///
/// Workers send periodic heartbeats. A worker is considered dead when
/// `heartbeat_interval_seconds × heartbeat_max_missed` seconds elapse with no
/// heartbeat while the worker is in `Computing` state. Dead workers cause their
/// job to be aborted.
///
/// ## Stale worker cleanup
///
/// Workers in `Disconnected` state for longer than `stale_disconnected_threshold_seconds`
/// are removed from the pool to prevent unbounded growth.
///
/// ## Completed job retention
///
/// Jobs in a terminal state (`Completed`, `Failed`, `Cancelled`) are kept in memory
/// for `job_ttl_seconds` after termination so clients can still query
/// their final state, then evicted by the monitor sweep. Set to `0` to disable
/// retention (jobs are removed on the next sweep after they terminate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Maximum number of workers that can be assigned to a single job.
    pub max_workers_per_job: u32,
    /// Maximum number of workers the coordinator will accept (across all jobs).
    pub max_total_workers: u32,
    /// Timeout for the Execution phase. Default: 300s.
    pub execution_timeout_seconds: u64,
    /// Timeout for Phase 1: Contributions. Default: 300s.
    pub phase1_timeout_seconds: u64,
    /// Timeout for Phase 2: Prove (proof generation). Default: 600s.
    pub phase2_timeout_seconds: u64,
    /// Timeout for Phase 3: Aggregate (proof aggregation). Default: 100s.
    pub phase3_timeout_seconds: u64,
    /// Expected interval between worker heartbeats. Default: 30s.
    pub heartbeat_interval_seconds: u64,
    /// Number of missed heartbeats before a computing worker is considered dead.
    /// Dead threshold = `heartbeat_interval_seconds × heartbeat_max_missed`. Default: 3.
    pub heartbeat_max_missed: u32,
    /// How often the background monitor sweeps for timeouts and stale heartbeats. Default: 10s.
    pub job_monitor_interval_seconds: u64,
    /// Seconds a worker can remain in `Disconnected` state before being removed from
    /// the pool entirely. Default: 300s.
    pub stale_disconnected_threshold_seconds: u64,
    /// Seconds a job in a terminal state (`Completed`, `Failed`, `Cancelled`) is kept
    /// in memory before being evicted by the monitor sweep. Default: 3600s (60 min).
    /// `0` disables retention.
    pub job_ttl_seconds: u64,
    /// Optional webhook URL to POST job completion/failure notifications.
    pub webhook_url: Option<String>,
    /// Default compute units for a job when the caller does not specify.
    /// `0` means "use all currently available capacity".
    pub default_compute_units: u32,
    /// Minimum compute units required to start any job. Jobs are rejected
    /// (`ResourceExhausted`) if available capacity falls below this floor.
    pub min_compute_units: u32,
    /// Grace period in milliseconds before a disconnected worker's job is failed.
    /// If the worker reconnects within this window the disconnect is treated as a
    /// transient network blip and computation continues uninterrupted.
    /// Default: 500 ms (suitable for same-datacenter clusters; increase for cross-DC).
    pub reconnect_grace_period_ms: u64,
}

impl Config {
    const DEFAULT_BIND_HOST: &'static str = "0.0.0.0";
    const DEFAULT_HOST: &'static str = "127.0.0.1";
    const DEFAULT_PORT: u16 = 50051;
    const DEFAULT_PROOFS_DIR: &'static str = "proofs";

    pub fn load(
        config_file: Option<String>,
        port: Option<u16>,
        proofs_dir: Option<PathBuf>,
        no_save_proofs: bool,
        webhook_url: Option<String>,
    ) -> Result<Self> {
        // Create proofs directory if it doesn't exist
        if let Some(ref path) = proofs_dir {
            if !path.exists() {
                std::fs::create_dir_all(path)?;
            } else if !path.is_dir() {
                anyhow::bail!("Proofs path exists but is not a directory: {}", path.display());
            }
        }

        let mut builder = config::Config::builder()
            .set_default("service.name", "ZisK Distributed Coordinator")?
            .set_default("service.version", env!("CARGO_PKG_VERSION"))?
            .set_default("service.environment", "development")?
            .set_default("server.host", Self::DEFAULT_BIND_HOST)?
            .set_default("server.port", Self::DEFAULT_PORT)?
            .set_default("server.proofs_dir", Self::DEFAULT_PROOFS_DIR)?
            .set_default("server.no_save_proofs", false)?
            .set_default("server.shutdown_timeout_seconds", 30)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?
            .set_default("coordinator.max_workers_per_job", 10)?
            .set_default("coordinator.max_total_workers", 1000)?
            .set_default("coordinator.execution_timeout_seconds", 300)?
            .set_default("coordinator.phase1_timeout_seconds", 300)?
            .set_default("coordinator.phase2_timeout_seconds", 600)?
            .set_default("coordinator.phase3_timeout_seconds", 100)?
            .set_default("coordinator.heartbeat_interval_seconds", 30)?
            .set_default("coordinator.heartbeat_max_missed", 3)?
            .set_default("coordinator.job_monitor_interval_seconds", 10)?
            .set_default("coordinator.stale_disconnected_threshold_seconds", 300)?
            .set_default("coordinator.job_ttl_seconds", 3600)?
            .set_default("coordinator.default_compute_units", 0)?
            .set_default("coordinator.min_compute_units", 1)?
            .set_default("coordinator.reconnect_grace_period_ms", 500_u64)?;

        if let Some(path) = config_file {
            builder = builder.add_source(config::File::with_name(&path));
        }

        // Force version to always be the compiled version (cannot be overridden by config)
        builder = builder.set_override("service.version", env!("CARGO_PKG_VERSION"))?;

        // Override port if provided via function argument
        if let Some(port) = port {
            builder = builder.set_override("server.port", port)?;
        }

        // Override proofs_dir if provided via function argument
        if let Some(proofs_dir) = proofs_dir {
            builder = builder
                .set_override("server.proofs_dir", proofs_dir.to_string_lossy().to_string())?;
        }

        builder = builder.set_override("server.no_save_proofs", no_save_proofs)?;

        // Override webhook_url if provided via function argument
        if let Some(url) = webhook_url {
            builder = builder.set_override("coordinator.webhook_url", url)?;
        }

        let config = builder.build()?;

        Ok(config.try_deserialize()?)
    }

    pub fn default_url() -> String {
        format!("http://{}:{}", Self::DEFAULT_HOST, Self::DEFAULT_PORT)
    }
}
