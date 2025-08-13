use serde::{Deserialize, Serialize};

/// Prover ID wrapper for type safety
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProverId(String);

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComputeCapacity {
    pub compute_units: u32,
}

// Conversion from protobuf ComputeCapacity to core ComputeCapacity
impl From<consensus_api::ComputeCapacity> for ComputeCapacity {
    fn from(proto_caps: consensus_api::ComputeCapacity) -> Self {
        Self { compute_units: proto_caps.compute_units }
    }
}

impl From<ComputeCapacity> for consensus_api::ComputeCapacity {
    fn from(val: ComputeCapacity) -> Self {
        consensus_api::ComputeCapacity { compute_units: val.compute_units }
    }
}

impl std::fmt::Display for ComputeCapacity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} CU", self.compute_units)
    }
}
