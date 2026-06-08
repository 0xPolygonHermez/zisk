//! Loads the coordinator metric/route contract from `known-contract.json`.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawContract {
    metrics: Vec<String>,
    routes: Vec<String>,
}

/// Source of truth for known coordinator metrics and JSON API paths.
#[derive(Debug, Clone)]
pub struct Contract {
    pub metrics: BTreeSet<String>,
    pub routes: BTreeSet<String>,
}

impl Contract {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!(
                "missing {}; regenerate with \
                 `cargo run -p zisk-coordinator-contract -- --output {}` \
                 or `make contract` from distributed/deploy/grafana/",
                path.display(),
                path.display()
            ));
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        let raw: RawContract = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse contract {}: {}", path.display(), e))?;
        Ok(Self {
            metrics: raw.metrics.into_iter().collect(),
            routes: raw.routes.into_iter().collect(),
        })
    }

    #[cfg(test)]
    pub fn from_sets(metrics: BTreeSet<String>, routes: BTreeSet<String>) -> Self {
        Self { metrics, routes }
    }
}
