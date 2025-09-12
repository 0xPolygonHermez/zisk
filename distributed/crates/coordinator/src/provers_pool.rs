use distributed_common::{ComputeCapacity, Error, ProverId, ProverState, Result};
use distributed_config::CoordinatorConfig;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{coordinator_service::MessageSender, dto::CoordinatorMessageDto, ProverConnection};

pub struct ProversPool {
    /// Map of prover_id to ProverConnection
    pub provers: RwLock<HashMap<ProverId, ProverConnection>>,

    /// Configuration for the Provers Pool
    config: CoordinatorConfig,
}

impl ProversPool {
    /// Create a new ProversPool
    pub fn new(config: CoordinatorConfig) -> Self {
        Self { provers: RwLock::new(HashMap::new()), config }
    }

    pub async fn num_provers(&self) -> usize {
        self.provers.read().await.len()
    }

    pub async fn compute_capacity(&self) -> ComputeCapacity {
        let total_capacity: u32 =
            self.provers.read().await.values().map(|p| p.compute_capacity.compute_units).sum();
        ComputeCapacity { compute_units: total_capacity }
    }

    pub async fn register_prover(
        &self,
        prover_id: ProverId,
        compute_capacity: impl Into<ComputeCapacity>,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> Result<ProverId> {
        let connection =
            ProverConnection::new(prover_id.clone(), compute_capacity.into(), msg_sender);

        // Check if we've reached the maximum number of total provers
        let num_provers = self.num_provers().await;
        if num_provers >= self.config.max_total_provers as usize {
            return Err(Error::InvalidRequest(format!(
                "Maximum number of provers reached: {}/{}",
                num_provers, self.config.max_total_provers
            )));
        }

        self.provers.write().await.insert(prover_id.clone(), connection);

        info!("Registered prover: {} (total: {})", prover_id, num_provers + 1);

        Ok(prover_id)
    }

    pub async fn prover_state(&self, prover_id: &ProverId) -> Option<ProverState> {
        self.provers.read().await.get(prover_id).map(|p| p.state.clone())
    }

    pub async fn mark_provers_with_state(
        &self,
        prover_ids: &[ProverId],
        state: ProverState,
    ) -> Result<()> {
        for prover_id in prover_ids {
            self.mark_prover_with_state(prover_id, state.clone()).await?;
        }
        Ok(())
    }

    pub async fn mark_prover_with_state(
        &self,
        prover_id: &ProverId,
        state: ProverState,
    ) -> Result<()> {
        if let Some(prover) = self.provers.write().await.get_mut(prover_id) {
            prover.state = state;
        } else {
            return Err(Error::InvalidRequest(format!("Prover {prover_id} not found")));
        }

        Ok(())
    }

    /// Remove a prover connection
    pub async fn unregister_prover(&self, prover_id: &ProverId) -> Result<()> {
        self.provers.write().await.remove(prover_id).map(|_| ()).ok_or_else(|| {
            let msg = format!("Prover {prover_id} not found for removal");
            warn!("{}", msg);
            Error::InvalidRequest(msg)
        })
    }

    pub async fn send_message(
        &self,
        prover_id: &ProverId,
        message: CoordinatorMessageDto,
    ) -> Result<()> {
        if let Some(prover) = self.provers.read().await.get(prover_id) {
            prover.msg_sender.send(message).map_err(|e| {
                let msg = format!("Failed to send message to prover {prover_id}: {}", e);
                warn!("{}", msg);
                Error::Comm(msg)
            })
        } else {
            let msg = format!("Prover {prover_id} not found for sending message");
            warn!("{}", msg);
            Err(Error::InvalidRequest(msg))
        }
    }

    pub async fn update_last_heartbeat(&self, prover_id: &ProverId) -> Result<()> {
        if let Some(prover) = self.provers.write().await.get_mut(prover_id) {
            prover.update_last_heartbeat();
            Ok(())
        } else {
            let msg = format!("Prover {prover_id} not found for heartbeat update");
            warn!("{}", msg);
            Err(Error::InvalidRequest(msg))
        }
    }

    pub async fn partition_and_allocate_by_capacity(
        &self,
        required_compute_capacity: ComputeCapacity,
    ) -> Result<(Vec<ProverId>, Vec<Vec<u32>>)> {
        if required_compute_capacity.compute_units == 0 {
            return Err(Error::InvalidRequest(
                "Compute capacity must be greater than 0".to_string(),
            ));
        }

        let provers = self.provers.write().await;

        let available_provers: Vec<(&ProverId, &ProverConnection)> =
            provers.iter().filter(|(_, p)| matches!(p.state, ProverState::Idle)).collect();

        let available_capacity: u32 =
            available_provers.iter().map(|(_, p)| p.compute_capacity.compute_units).sum();

        if required_compute_capacity.compute_units > available_capacity {
            return Err(Error::InvalidRequest(format!(
                "Not enough compute capacity available: need {required_compute_capacity}, have {available_capacity}",
            )));
        }

        let mut selected_provers = Vec::new();
        let mut prover_capacities = Vec::new();
        let mut total_capacity = 0;

        // Step 1: Select provers that can cover the required compute capacity
        for (prover_id, prover_connection) in available_provers {
            if matches!(prover_connection.state, ProverState::Idle) {
                selected_provers.push(prover_id.clone());
                prover_capacities.push(prover_connection.compute_capacity.compute_units);
                total_capacity += prover_connection.compute_capacity.compute_units;

                println!(
                    "Prover {} capacity: {}",
                    prover_id, prover_connection.compute_capacity.compute_units
                );

                // Stop when we have enough capacity
                if total_capacity >= required_compute_capacity.compute_units {
                    break;
                }
            }
        }

        drop(provers);

        // Step 2: Assign partitions using round-robin
        let num_provers = selected_provers.len();
        let total_units = required_compute_capacity.compute_units;
        let mut prover_allocations = vec![Vec::new(); num_provers];

        // Round-robin assignment of compute units
        for unit in 0..total_units {
            let prover_idx = (unit as usize) % num_provers;

            // Check if this prover still has capacity
            if prover_allocations[prover_idx].len() < prover_capacities[prover_idx] as usize {
                prover_allocations[prover_idx].push(unit);
            } else {
                // If this prover is at capacity, find the next available prover
                let mut found = false;
                for offset in 1..num_provers {
                    let next_idx = (prover_idx + offset) % num_provers;
                    if prover_allocations[next_idx].len() < prover_capacities[next_idx] as usize {
                        prover_allocations[next_idx].push(unit);
                        found = true;
                        break;
                    }
                }

                if !found {
                    warn!("Could not assign compute unit {} to any prover", unit);
                    break;
                }
            }
        }

        Ok((selected_provers, prover_allocations))
    }

    pub async fn select_agg_prover(&self) -> Vec<ProverId> {
        let available_provers = self.provers.read().await;
        // For the sake of simplicity, we use now only the first prover to aggregate the proofs
        vec![available_provers.iter().next().unwrap().0.clone()]
    }
}
