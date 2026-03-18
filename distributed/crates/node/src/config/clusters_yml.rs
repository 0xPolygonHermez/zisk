use crate::errors::{NodeError, NodeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level clusters.yml structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClustersFile {
    #[serde(default)]
    pub machines: HashMap<String, MachineEntry>,
    #[serde(default)]
    pub clusters: HashMap<String, ClusterEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineEntry {
    /// Hostname or IP of the node daemon on this machine.
    pub node: String,
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
    pub machine: String,
    pub instance: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEntry {
    pub machine: String,
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
        // Per-machine: track which cluster has claimed each GPU
        let mut gpu_owners: HashMap<(&str, u32), &str> = HashMap::new();
        // Coordinator ports must be unique per machine
        let mut coord_ports: HashMap<(&str, u16), &str> = HashMap::new();
        // Instance names must be unique per machine
        let mut instances: HashMap<(&str, &str), &str> = HashMap::new();

        for (cluster_name, cluster) in &self.clusters {
            let coord_machine = &cluster.coordinator.machine;
            if !self.machines.contains_key(coord_machine) {
                return Err(NodeError::Validation(format!(
                    "cluster '{cluster_name}': coordinator machine '{coord_machine}' not found in machines"
                )));
            }

            let port_key = (coord_machine.as_str(), cluster.coordinator.port);
            if let Some(prev) = coord_ports.insert(port_key, cluster_name) {
                return Err(NodeError::Validation(format!(
                    "cluster '{cluster_name}': coordinator port {} on machine '{}' already used by cluster '{prev}'",
                    cluster.coordinator.port, coord_machine
                )));
            }

            let inst_key = (coord_machine.as_str(), cluster.coordinator.instance.as_str());
            if let Some(prev) = instances.insert(inst_key, cluster_name) {
                return Err(NodeError::Validation(format!(
                    "cluster '{cluster_name}': instance '{}' on machine '{}' already used by cluster '{prev}'",
                    cluster.coordinator.instance, coord_machine
                )));
            }

            for worker in &cluster.workers {
                if !self.machines.contains_key(&worker.machine) {
                    return Err(NodeError::Validation(format!(
                        "cluster '{cluster_name}': worker '{}' machine '{}' not found in machines",
                        worker.instance, worker.machine
                    )));
                }

                let w_inst_key = (worker.machine.as_str(), worker.instance.as_str());
                if let Some(prev) = instances.insert(w_inst_key, cluster_name) {
                    return Err(NodeError::Validation(format!(
                        "cluster '{cluster_name}': instance '{}' on machine '{}' already used by cluster '{prev}'",
                        worker.instance, worker.machine
                    )));
                }

                for &gpu_id in &worker.gpus {
                    let gpu_key = (worker.machine.as_str(), gpu_id);
                    if let Some(prev_cluster) = gpu_owners.insert(gpu_key, cluster_name) {
                        return Err(NodeError::Validation(format!(
                            "cluster '{cluster_name}': GPU {} on machine '{}' already assigned to cluster '{prev_cluster}'",
                            gpu_id, worker.machine
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}
