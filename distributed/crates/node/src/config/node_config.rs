use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub service: ServiceConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub node: NodeSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub version: String,
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
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSettings {
    /// Path to clusters.yml (only read on head nodes)
    pub clusters_file: Option<PathBuf>,
    /// Advertised address for this node (used by peers)
    pub advertise_addr: Option<String>,
    /// Working directory for spawned coordinator/worker processes
    pub work_dir: PathBuf,
}

impl NodeConfig {
    const DEFAULT_HOST: &'static str = "0.0.0.0";
    const DEFAULT_PORT: u16 = 7000;
    const DEFAULT_WORK_DIR: &'static str = "/var/lib/zisk";

    pub fn load(config_file: Option<String>, port: Option<u16>) -> Result<Self> {
        let mut builder = config::Config::builder()
            .set_default("service.name", "ZisK Node")?
            .set_default("service.version", env!("CARGO_PKG_VERSION"))?
            .set_default("server.host", Self::DEFAULT_HOST)?
            .set_default("server.port", Self::DEFAULT_PORT)?
            .set_default("server.shutdown_timeout_seconds", 30)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?
            .set_default("node.work_dir", Self::DEFAULT_WORK_DIR)?;

        if let Some(path) = config_file {
            builder = builder.add_source(config::File::with_name(&path));
        }

        builder = builder.set_override("service.version", env!("CARGO_PKG_VERSION"))?;

        if let Some(p) = port {
            builder = builder.set_override("server.port", p)?;
        }

        Ok(builder.build()?.try_deserialize()?)
    }

    pub fn default_url() -> String {
        format!("http://127.0.0.1:{}", Self::DEFAULT_PORT)
    }
}
