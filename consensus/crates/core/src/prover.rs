use serde::{Deserialize, Serialize};

/// Prover ID wrapper for type safety
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProverId(pub String);

impl Default for ProverId {
    fn default() -> Self {
        Self::new()
    }
}

impl ProverId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for ProverId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<ProverId> for String {
    fn from(prover_id: ProverId) -> Self {
        prover_id.0
    }
}

impl std::fmt::Display for ProverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProverId({})", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverCapabilities {
    pub cpu_cores_num: u32,
    pub gpu_num: u32,
}

// Conversion from protobuf ProverCapabilities to core ProverCapabilities
impl From<consensus_api::ProverCapabilities> for ProverCapabilities {
    fn from(proto_caps: consensus_api::ProverCapabilities) -> Self {
        Self { cpu_cores_num: proto_caps.cpu_cores_num, gpu_num: proto_caps.gpu_num }
    }
}
