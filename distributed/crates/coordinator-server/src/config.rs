//! Coordinator server configuration — loaded from TOML, with env-var and CLI overrides.
//!
//! Load order (later entries override earlier):
//! 1. Built-in defaults
//! 2. TOML file (path from `--config` or `ZISK_COORDINATOR_CONFIG`)
//! 3. CLI flags / env vars: --api-port, --cluster-port, --metrics-port, --log-level

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zisk_cluster_common::{Environment, LoggingConfig};

/// Top-level coordinator-server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Service identity (name, version, environment).
    pub service: ServiceConfig,
    /// gRPC API server settings.
    pub server: ServerConfig,
    /// Prometheus metrics endpoint settings.
    pub metrics: MetricsConfig,
    /// Logging configuration.
    pub logging: LoggingConfig,
    /// Which backend implementation to run.
    pub backend: BackendConfig,
    /// Settings for the embedded coordinator core.
    pub coordinator: CoordinatorConfig,
}

/// Service identity metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Human-readable service name.
    pub name: String,
    /// Service version (defaults to the crate version).
    pub version: String,
    /// Deployment environment (development, production, …).
    pub environment: Environment,
}

/// gRPC API server settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind host for the API server.
    pub host: String,
    /// Bind port for the API server.
    pub port: u16,
    /// Grace period, in seconds, to wait for in-flight work on shutdown.
    pub shutdown_timeout_seconds: u64,
}

/// Prometheus metrics endpoint settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether the metrics endpoint is served.
    pub enabled: bool,
    /// Bind host for the metrics endpoint.
    pub host: String,
    /// Bind port for the metrics endpoint.
    pub port: u16,
}

/// Selects which backend the server runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// The backend mode.
    pub mode: BackendMode,
}

/// Backend implementation to run behind the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendMode {
    /// In-memory mock backend (no coordinator); for testing.
    Mock,
    /// In-process coordinator core.
    Coordinator,
}

/// Config section for the coordinator core that runs in-process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Path to a coordinator TOML config file. `None` uses coordinator defaults.
    pub config_file: Option<String>,
    /// Port on which the embedded coordinator listens for worker connections.
    pub port: u16,
}

impl Config {
    /// Load the configuration, applying (in increasing priority) built-in
    /// defaults, well-known and explicit TOML files, then the given CLI/env
    /// overrides.
    pub fn load(
        config_file: Option<String>,
        api_port: Option<u16>,
        cluster_port: Option<u16>,
        metrics_port: Option<u16>,
        log_level: Option<String>,
    ) -> Result<Self> {
        let mut builder = config::Config::builder()
            // service
            .set_default("service.name", "ZisK Coordinator")?
            .set_default("service.version", env!("CARGO_PKG_VERSION"))?
            .set_default("service.environment", "development")?
            // server
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 7000)?
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
            .set_default("coordinator.port", 50051u16)?;

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
        if let Some(level) = log_level {
            builder = builder.set_override("logging.level", level)?;
        }

        Ok(builder.build()?.try_deserialize()?)
    }

    /// The `host:port` address the gRPC API server binds to.
    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// The `host:port` address the metrics endpoint binds to.
    pub fn metrics_addr(&self) -> String {
        format!("{}:{}", self.metrics.host, self.metrics.port)
    }
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
        let cfg = Config::load(None, None, None, None, None).unwrap();
        assert_eq!(cfg.server.host, "0.0.0.0");
        assert_eq!(cfg.server.port, 7000);
        assert_eq!(cfg.coordinator.port, 50051);
        assert_eq!(cfg.metrics.port, 9090);
        assert_eq!(cfg.backend.mode, BackendMode::Coordinator);
        assert_eq!(cfg.service.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn cli_api_port_override() {
        let cfg = Config::load(None, Some(8080), None, None, None).unwrap();
        assert_eq!(cfg.server.port, 8080);
    }

    #[test]
    fn cli_cluster_port_override() {
        let cfg = Config::load(None, None, Some(50100), None, None).unwrap();
        assert_eq!(cfg.coordinator.port, 50100);
    }

    #[test]
    fn cli_metrics_port_override() {
        let cfg = Config::load(None, None, None, Some(9999), None).unwrap();
        assert_eq!(cfg.metrics.port, 9999);
    }

    #[test]
    fn grpc_addr_format() {
        let cfg = Config::load(None, Some(9000), None, None, None).unwrap();
        assert_eq!(cfg.grpc_addr(), "0.0.0.0:9000");
    }
}
