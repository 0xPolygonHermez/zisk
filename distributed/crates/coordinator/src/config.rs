use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use zisk_distributed_common::Environment;
use zisk_distributed_common::LoggingConfig;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub max_workers_per_job: u32,
    pub max_total_workers: u32,
    pub phase1_timeout_seconds: u64,
    pub phase2_timeout_seconds: u64,
    pub webhook_url: Option<String>,
    pub compressed_proofs: bool,
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
        compressed_proofs: bool,
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
            .set_default("coordinator.phase1_timeout_seconds", 300)?
            .set_default("coordinator.phase2_timeout_seconds", 600)?
            .set_default("coordinator.compressed_proofs", compressed_proofs)?;

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
