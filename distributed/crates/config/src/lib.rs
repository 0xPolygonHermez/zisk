use serde::{Deserialize, Serialize};
use std::env;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub service: ServiceConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub comm: CommConfig,
    pub prover_manager: ProverManagerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub version: String,
    pub build_time: String,
    pub commit_hash: String,
    pub environment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shutdown_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub file_output: bool,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "pretty")]
    Pretty,
    #[serde(rename = "compact")]
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommConfig {
    pub max_peers: usize,
    pub discovery_interval_seconds: u64,
    pub heartbeat_interval_seconds: u64,
    pub connection_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverManagerConfig {
    pub max_provers_per_job: u32,
    pub max_total_provers: u32,
    pub max_concurrent_connections: u32,
    pub message_buffer_size: u32,
    pub phase1_timeout_seconds: u64,
    pub phase2_timeout_seconds: u64,
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut builder = config::Config::builder()
            .set_default("service.name", "consensus-network")?
            .set_default("service.version", env!("CARGO_PKG_VERSION"))?
            .set_default("service.build_time", build_time())?
            .set_default("service.commit_hash", commit_hash())?
            .set_default("service.environment", "development")?
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.shutdown_timeout_seconds", 30)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?
            .set_default("logging.file_output", false)?
            .set_default("comm.max_peers", 100)?
            .set_default("comm.discovery_interval_seconds", 30)?
            .set_default("comm.heartbeat_interval_seconds", 10)?
            .set_default("comm.connection_timeout_seconds", 30)?
            .set_default("prover_manager.max_provers_per_job", 10)?
            .set_default("prover_manager.max_total_provers", 1000)?
            .set_default("prover_manager.max_concurrent_connections", 500)?
            .set_default("prover_manager.message_buffer_size", 1000)?
            .set_default("prover_manager.phase1_timeout_seconds", 300)?
            .set_default("prover_manager.phase2_timeout_seconds", 600)?;

        // Load from config file if it exists
        if let Ok(config_path) = env::var("CONFIG_PATH") {
            builder = builder.add_source(config::File::with_name(&config_path));
        } else {
            // Try default config file locations
            builder = builder
                .add_source(config::File::with_name("config/default").required(false))
                .add_source(config::File::with_name("config/local").required(false));
        }

        // Override with environment variables (with CONSENSUS_ prefix)
        builder = builder.add_source(
            config::Environment::with_prefix("CONSENSUS").separator("_").try_parsing(true),
        );

        let config = builder.build()?;
        Ok(config.try_deserialize()?)
    }
}

fn build_time() -> String {
    env::var("BUILD_TIME").unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
}

fn commit_hash() -> String {
    env::var("GIT_COMMIT_HASH").unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::load().unwrap();
        assert_eq!(config.service.name, "consensus-network");
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
    }
}
