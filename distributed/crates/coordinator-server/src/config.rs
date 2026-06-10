//! Coordinator server configuration — loaded from TOML, with env-var and CLI overrides.
//!
//! Load order (later entries override earlier):
//! 1. Built-in defaults
//! 2. TOML file (path from `--config` or `ZISK_COORDINATOR_CONFIG`)
//! 3. CLI flags / env vars: --api-port, --cluster-port, --metrics-port,
//!    --coordinator-id, --log-level

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use zisk_cluster_common::{Environment, LoggingConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub service: ServiceConfig,
    pub server: ServerConfig,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
    pub backend: BackendConfig,
    pub coordinator: CoordinatorConfig,
    pub job_history: JobHistoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub coordinator_id: String,
    pub name: String,
    pub version: String,
    pub environment: Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shutdown_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub mode: BackendMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendMode {
    Mock,
    Coordinator,
}

/// Config section for the coordinator core that runs in-process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Path to a coordinator TOML config file. `None` uses coordinator defaults.
    pub config_file: Option<String>,
    /// Port on which the embedded coordinator listens for worker connections.
    pub port: u16,
    /// When `Some(true)`, completed proofs are persisted to disk by the
    /// coordinator. `None` leaves the embedded coordinator's own default in
    /// place (no save).
    pub save_proofs: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistoryConfig {
    pub enabled: bool,
    /// Postgres connection URL. The env var `ZISK_COORDINATOR_DATABASE_URL`
    /// takes precedence so production deployments keep credentials out of the
    /// committed config file. The TOML value is the dev / local-stack default
    /// that lets `coord up + docker compose up` work without env-var dance.
    #[serde(default)]
    pub database_url: Option<String>,
    pub auto_migrate: bool,
    pub channel_capacity: usize,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
}

impl Config {
    pub fn load(
        config_file: Option<String>,
        api_port: Option<u16>,
        cluster_port: Option<u16>,
        metrics_port: Option<u16>,
        coordinator_id: Option<String>,
        log_level: Option<String>,
    ) -> Result<Self> {
        let mut builder = config::Config::builder()
            // service
            .set_default("service.coordinator_id", "default")?
            .set_default("service.name", "ZisK Coordinator")?
            .set_default("service.version", env!("CARGO_PKG_VERSION"))?
            .set_default("service.environment", "development")?
            // server
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 15100)?
            .set_default("server.shutdown_timeout_seconds", 30)?
            // metrics
            .set_default("metrics.enabled", true)?
            .set_default("metrics.host", "0.0.0.0")?
            .set_default("metrics.port", 9090)?
            // logging
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?
            // backend
            .set_default("backend.mode", "coordinator")?
            // coordinator
            .set_default("coordinator.port", 50051u16)?
            // job history
            .set_default("job_history.enabled", false)?
            .set_default("job_history.auto_migrate", true)?
            .set_default("job_history.channel_capacity", 10_000u64)?
            .set_default("job_history.batch_size", 100u64)?
            .set_default("job_history.flush_interval_ms", 250u64)?;

        // Well-known config file locations, searched in order (least to most specific).
        for path in default_config_paths() {
            builder = builder
                .add_source(config::File::with_name(&path.to_string_lossy()).required(false));
        }

        // Explicit --config / ZISK_COORDINATOR_CONFIG overrides the well-known paths.
        if let Some(path) = config_file {
            builder = builder.add_source(config::File::with_name(&path));
        }

        // CLI / env-var overrides — always highest priority.
        // Each field has an explicit env var defined on the clap arg in main.rs.
        builder = builder.set_override("service.version", env!("CARGO_PKG_VERSION"))?;
        if let Some(p) = api_port {
            builder = builder.set_override("server.port", p)?;
        }
        if let Some(p) = cluster_port {
            builder = builder.set_override("coordinator.port", p)?;
        }
        if let Some(p) = metrics_port {
            builder = builder.set_override("metrics.port", p)?;
        }
        if let Some(id) = coordinator_id {
            builder = builder.set_override("service.coordinator_id", id)?;
        }
        if let Some(level) = log_level {
            builder = builder.set_override("logging.level", level)?;
        }

        let mut cfg: Self = builder.build()?.try_deserialize()?;
        cfg.service.coordinator_id = normalize_coordinator_id(&cfg.service.coordinator_id)?;
        Ok(cfg)
    }

    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn metrics_addr(&self) -> String {
        format!("{}:{}", self.metrics.host, self.metrics.port)
    }
}

fn normalize_coordinator_id(id: &str) -> Result<String> {
    let id = id.trim();
    if id.is_empty() {
        bail!("coordinator_id must not be empty");
    }
    if id.len() > 128 {
        bail!("coordinator_id must be 128 bytes or less");
    }
    if !id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':')) {
        bail!("coordinator_id may only contain ASCII letters, digits, '-', '_', '.', or ':'");
    }
    Ok(id.to_owned())
}

/// Returns the well-known coordinator config file paths, ordered from least to most specific.
///
/// Search order:
/// 1. `/etc/zisk/coordinator.toml`        — system-wide
/// 2. `$XDG_CONFIG_HOME/zisk/coordinator.toml` — user-level (falls back to `~/.config/`)
/// 3. `./coordinator.toml`                — current directory (dev / project-local)
fn default_config_paths() -> Vec<std::path::PathBuf> {
    let mut paths = vec![std::path::PathBuf::from("/etc/zisk/coordinator.toml")];

    let xdg_base =
        std::env::var("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| std::path::PathBuf::from(".config"))
        });
    paths.push(xdg_base.join("zisk").join("coordinator.toml"));

    paths.push(std::path::PathBuf::from("coordinator.toml"));

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_load_without_file() {
        let cfg = Config::load(None, None, None, None, None, None).unwrap();
        assert_eq!(cfg.server.host, "0.0.0.0");
        assert_eq!(cfg.server.port, 15100);
        assert_eq!(cfg.coordinator.port, 50051);
        assert_eq!(cfg.metrics.port, 9090);
        assert_eq!(cfg.backend.mode, BackendMode::Coordinator);
        assert_eq!(cfg.service.coordinator_id, "default");
        assert!(!cfg.job_history.enabled);
        assert_eq!(cfg.job_history.channel_capacity, 10_000);
        assert_eq!(cfg.service.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn cli_api_port_override() {
        let cfg = Config::load(None, Some(8080), None, None, None, None).unwrap();
        assert_eq!(cfg.server.port, 8080);
    }

    #[test]
    fn cli_cluster_port_override() {
        let cfg = Config::load(None, None, Some(50100), None, None, None).unwrap();
        assert_eq!(cfg.coordinator.port, 50100);
    }

    #[test]
    fn cli_metrics_port_override() {
        let cfg = Config::load(None, None, None, Some(9999), None, None).unwrap();
        assert_eq!(cfg.metrics.port, 9999);
    }

    #[test]
    fn coordinator_id_override_is_normalized() {
        let cfg =
            Config::load(None, None, None, None, Some(" coord-a.1 ".to_string()), None).unwrap();
        assert_eq!(cfg.service.coordinator_id, "coord-a.1");
    }

    #[test]
    fn coordinator_id_rejects_bad_label_value() {
        assert!(Config::load(None, None, None, None, Some("bad/id".to_string()), None).is_err());
    }

    #[test]
    fn grpc_addr_format() {
        let cfg = Config::load(None, Some(9000), None, None, None, None).unwrap();
        assert_eq!(cfg.grpc_addr(), "0.0.0.0:9000");
    }
}
