use serde::{Deserialize, Serialize};
use std::env;

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
    pub shutdown_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub version: String,
    pub environment: String,
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
pub struct CoordinatorConfig {
    pub max_workers_per_job: u32,
    pub max_total_workers: u32,
    pub phase1_timeout_seconds: u64,
    pub phase2_timeout_seconds: u64,
    pub webhook_url: Option<String>,
}

impl Config {
    pub fn load(port: Option<u16>, webhook_url: Option<String>) -> Result<Self> {
        let mut builder = config::Config::builder()
            .set_default("service.name", "ZisK Distributed Coordinator")?
            .set_default("service.version", env!("CARGO_PKG_VERSION"))?
            .set_default("service.environment", "development")?
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.shutdown_timeout_seconds", 30)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?
            .set_default("logging.file_output", false)?
            .set_default("coordinator.max_workers_per_job", 10)?
            .set_default("coordinator.max_total_workers", 1000)?
            .set_default("coordinator.phase1_timeout_seconds", 300)?
            .set_default("coordinator.phase2_timeout_seconds", 600)?;

        // Load from config file if it exists
        if let Ok(config_path) = env::var("CONFIG_PATH") {
            builder = builder.add_source(config::File::with_name(&config_path));
        } else {
            // Try default config file locations
            builder = builder
                .add_source(config::File::with_name("config/default").required(false))
                .add_source(config::File::with_name("config/local").required(false));
        }

        // Override with environment variables (with DISTRIBUTED_ prefix)
        builder = builder.add_source(
            config::Environment::with_prefix("DISTRIBUTED").separator("_").try_parsing(true),
        );

        // Override port if provided via function argument
        if let Some(port) = port {
            builder = builder.set_override("server.port", port)?;
        }

        // Override webhook_url if provided via function argument
        if let Some(url) = webhook_url {
            builder = builder.set_override("coordinator.webhook_url", url)?;
        }

        let config = builder.build()?;

        Ok(config.try_deserialize()?)
    }
}
