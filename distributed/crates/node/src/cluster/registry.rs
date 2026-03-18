use crate::config::clusters_yml::{ClusterEntry, MachineEntry};
use crate::errors::{NodeError, NodeResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Read-only view of the single cluster loaded from clusters.yml at startup.
pub struct ClusterRegistry {
    name: String,
    cluster: ClusterEntry,
    machines: HashMap<String, MachineEntry>,
}

impl ClusterRegistry {
    /// Load clusters.yml and validate that exactly one cluster is defined.
    pub fn load(path: PathBuf) -> NodeResult<Arc<Self>> {
        let file = crate::config::clusters_yml::ClustersFile::load(&path)?;
        if file.clusters.len() != 1 {
            return Err(NodeError::Validation(format!(
                "expected exactly one cluster in '{}', found {}",
                path.display(),
                file.clusters.len()
            )));
        }
        let (name, cluster) = file.clusters.into_iter().next().unwrap();
        info!("Loaded cluster '{name}'");
        Ok(Arc::new(Self { name, cluster, machines: file.machines }))
    }

    pub fn cluster_name(&self) -> &str {
        &self.name
    }

    pub fn cluster(&self) -> &ClusterEntry {
        &self.cluster
    }

    /// Resolves the coordinator gRPC URL from the cluster config.
    /// Format: `http://{machine.node}:{coordinator.port}`
    pub fn coordinator_url(&self) -> NodeResult<String> {
        let coord = &self.cluster.coordinator;
        let machine = self.machines.get(&coord.machine).ok_or_else(|| {
            NodeError::Validation(format!(
                "coordinator machine '{}' not found in machines",
                coord.machine
            ))
        })?;
        Ok(format!("http://{}:{}", machine.node, coord.port))
    }
}
