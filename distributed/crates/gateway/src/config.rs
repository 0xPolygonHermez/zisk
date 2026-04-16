//! Gateway configuration — loaded from TOML, with env-var and CLI overrides.
//!
//! Load order (later entries override earlier):
//! 1. Built-in defaults
//! 2. TOML file (path from `--config` or `ZISK_GATEWAY_CONFIG`)
//! 3. `ZISK_GATEWAY_*` environment variables
//! 4. CLI flags passed explicitly to [`Config::load`]

use anyhow::Result;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
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

/// Config section for the coordinator that runs in-process with the gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Path to a coordinator TOML config file. `None` uses coordinator defaults.
    pub config_file: Option<String>,
    /// Port on which the embedded coordinator listens for worker connections.
    pub worker_port: u16,
}

impl Config {
    pub fn load(
        config_file: Option<String>,
        port: Option<u16>,
        log_level: Option<String>,
        backend: Option<String>,
    ) -> Result<Self> {
        let mut builder = config::Config::builder()
            // service
            .set_default("service.name", "ZisK Gateway")?
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
            .set_default("backend.mode", "mock")?
            // coordinator
            .set_default("coordinator.worker_port", 50051u16)?;

        // Well-known config file locations, searched in order (least to most specific).
        // All are optional — the first one found wins for any given key.
        for path in default_config_paths() {
            builder = builder
                .add_source(config::File::with_name(&path.to_string_lossy()).required(false));
        }

        // Explicit --config / ZISK_GATEWAY_CONFIG overrides the well-known paths.
        if let Some(path) = config_file {
            builder = builder.add_source(config::File::with_name(&path));
        }

        // Environment variable overrides: ZISK_GATEWAY__SERVER__PORT etc.
        builder = builder.add_source(
            config::Environment::with_prefix("ZISK_GATEWAY").separator("__").try_parsing(true),
        );

        // CLI overrides — always highest priority
        builder = builder.set_override("service.version", env!("CARGO_PKG_VERSION"))?;
        if let Some(p) = port {
            builder = builder.set_override("server.port", p)?;
        }
        if let Some(level) = log_level {
            builder = builder.set_override("logging.level", level)?;
        }
        if let Some(mode) = backend {
            builder = builder.set_override("backend.mode", mode)?;
        }

        Ok(builder.build()?.try_deserialize()?)
    }

    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn metrics_addr(&self) -> String {
        format!("{}:{}", self.metrics.host, self.metrics.port)
    }
}

/// Returns the well-known gateway config file paths, ordered from least to most specific.
///
/// Search order:
/// 1. `/etc/zisk/gateway.toml`        — system-wide
/// 2. `$XDG_CONFIG_HOME/zisk/gateway.toml` — user-level (falls back to `~/.config/`)
/// 3. `./gateway.toml`                — current directory (dev / project-local)
fn default_config_paths() -> Vec<std::path::PathBuf> {
    let mut paths = vec![std::path::PathBuf::from("/etc/zisk/gateway.toml")];

    let xdg_base =
        std::env::var("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| std::path::PathBuf::from(".config"))
        });
    paths.push(xdg_base.join("zisk").join("gateway.toml"));

    paths.push(std::path::PathBuf::from("gateway.toml"));

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_load_without_file() {
        let cfg = Config::load(None, None, None, None).unwrap();
        assert_eq!(cfg.server.port, 7000);
        assert_eq!(cfg.metrics.port, 9090);
        assert_eq!(cfg.backend.mode, BackendMode::Mock);
        assert_eq!(cfg.service.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn cli_port_override() {
        let cfg = Config::load(None, Some(8080), None, None).unwrap();
        assert_eq!(cfg.server.port, 8080);
    }

    #[test]
    fn grpc_addr_format() {
        let cfg = Config::load(None, Some(9000), None, None).unwrap();
        assert_eq!(cfg.grpc_addr(), "0.0.0.0:9000");
    }
}
