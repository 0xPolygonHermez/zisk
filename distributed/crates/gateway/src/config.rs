//! Gateway configuration — loaded from TOML, with env-var and CLI overrides.
//!
//! Load order (later entries override earlier):
//! 1. Built-in defaults
//! 2. TOML file (path from `--config` or `ZISK_GATEWAY_CONFIG`)
//! 3. `ZISK_GATEWAY_*` environment variables
//! 4. CLI flags passed explicitly to [`Config::load`]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zisk_distributed_common::{Environment, LoggingConfig};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub url: String,
    pub connect_timeout_seconds: u64,
    pub request_timeout_seconds: u64,
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
            // coordinator (used only when backend.mode = "coordinator")
            .set_default("coordinator.url", "http://127.0.0.1:50051")?
            .set_default("coordinator.connect_timeout_seconds", 5)?
            .set_default("coordinator.request_timeout_seconds", 30)?;

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
