use anyhow::Result;
use consensus_core::{ComputeCapacity, ProverId};
use serde::{Deserialize, Serialize};

/// Client configuration structure that can be loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverGrpcEndpointConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Prover configuration
    pub prover: ProverConfig,

    /// Connection configuration
    #[serde(default)]
    pub connection: ConnectionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server URL to connect to
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverConfig {
    /// Prover ID (optional, will auto-generate if not provided)
    pub prover_id: ProverId,

    /// Compute capacity configuration
    pub compute_capacity: ComputeCapacity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Reconnection interval in seconds
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval_seconds: u64,

    /// Heartbeat timeout in seconds
    #[serde(default = "default_heartbeat_timeout")]
    pub heartbeat_timeout_seconds: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            reconnect_interval_seconds: default_reconnect_interval(),
            heartbeat_timeout_seconds: default_heartbeat_timeout(),
        }
    }
}

fn default_reconnect_interval() -> u64 {
    5
}

fn default_heartbeat_timeout() -> u64 {
    30
}

impl ProverGrpcEndpointConfig {
    /// Load configuration from a specific file
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ProverGrpcEndpointConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Apply CLI overrides to the configuration
    pub fn apply_cli_overrides(
        &mut self,
        url: Option<String>,
        prover_id: Option<String>,
        compute_units: Option<u32>,
    ) {
        if let Some(url) = url {
            self.server.url = url;
        }

        if let Some(prover_id) = prover_id {
            self.prover.prover_id = ProverId::from(prover_id);
        }

        if let Some(compute_units) = compute_units {
            self.prover.compute_capacity.compute_units = compute_units;
        }
    }

    /// Get the prover ID, generating one if not set
    pub fn get_prover_id(&self) -> ProverId {
        self.prover.prover_id.clone()
    }
}
