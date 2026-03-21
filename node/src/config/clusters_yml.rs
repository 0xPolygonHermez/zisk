use crate::errors::{NodeError, NodeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level clusters.yml structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClustersFile {
    #[serde(default)]
    pub nodes: HashMap<String, NodeEntry>,
    #[serde(default)]
    pub clusters: HashMap<String, ClusterEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEntry {
    /// Hostname or IP of the node daemon on this machine.
    pub address: String,
    pub port: u16,
    #[serde(default)]
    pub gpus: Vec<GpuEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuEntry {
    pub id: u32,
    pub memory_gb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterEntry {
    pub coordinator: CoordinatorEntry,
    #[serde(default)]
    pub workers: Vec<WorkerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorEntry {
    pub node: String,
    pub instance: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEntry {
    pub node: String,
    pub instance: String,
    pub port: u16,
    #[serde(default)]
    pub gpus: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch: Option<LaunchConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub mode: LaunchMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub np: Option<u32>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchMode {
    Direct,
    Mpi,
}

impl ClustersFile {
    pub fn load(path: &Path) -> NodeResult<Self> {
        let content = std::fs::read_to_string(path).map_err(NodeError::Io)?;
        let file: ClustersFile = serde_yaml::from_str(&content)?;
        file.validate()?;
        Ok(file)
    }

    /// Validate referential integrity and GPU uniqueness.
    pub fn validate(&self) -> NodeResult<()> {
        // Per-node: track which cluster has claimed each GPU
        let mut gpu_owners: HashMap<(&str, u32), &str> = HashMap::new();
        // Coordinator ports must be unique per node
        let mut coord_ports: HashMap<(&str, u16), &str> = HashMap::new();

        for (cluster_name, cluster) in &self.clusters {
            // Instance names must be unique within this cluster
            let mut instances: HashMap<&str, &str> = HashMap::new();

            let coord_node = &cluster.coordinator.node;
            if !self.nodes.contains_key(coord_node) {
                return Err(NodeError::Validation(format!(
                    "cluster '{cluster_name}': coordinator node '{coord_node}' not found in nodes"
                )));
            }

            let port_key = (coord_node.as_str(), cluster.coordinator.port);
            if let Some(prev) = coord_ports.insert(port_key, cluster_name) {
                return Err(NodeError::Validation(format!(
                    "cluster '{cluster_name}': coordinator port {} on node '{}' already used by cluster '{prev}'",
                    cluster.coordinator.port, coord_node
                )));
            }

            let coord_instance = cluster.coordinator.instance.as_str();
            if let Some(prev) = instances.insert(coord_instance, cluster_name) {
                return Err(NodeError::Validation(format!(
                    "cluster '{cluster_name}': instance '{coord_instance}' already used by '{prev}'"
                )));
            }

            for worker in &cluster.workers {
                if !self.nodes.contains_key(&worker.node) {
                    return Err(NodeError::Validation(format!(
                        "cluster '{cluster_name}': worker '{}' node '{}' not found in nodes",
                        worker.instance, worker.node
                    )));
                }

                let w_instance = worker.instance.as_str();
                if let Some(prev) = instances.insert(w_instance, cluster_name) {
                    return Err(NodeError::Validation(format!(
                        "cluster '{cluster_name}': instance '{w_instance}' already used by '{prev}'"
                    )));
                }

                for &gpu_id in &worker.gpus {
                    let gpu_key = (worker.node.as_str(), gpu_id);
                    if let Some(prev_cluster) = gpu_owners.insert(gpu_key, cluster_name) {
                        return Err(NodeError::Validation(format!(
                            "cluster '{cluster_name}': GPU {} on node '{}' already assigned to cluster '{prev_cluster}'",
                            gpu_id, worker.node
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}
