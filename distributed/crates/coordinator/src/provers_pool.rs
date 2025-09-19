use distributed_common::{
    ComputeCapacity, CoordinatorMessageDto, JobExecutionMode, ProverId, ProverInfoDto, ProverState,
    ProversListDto,
};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    coordinator_service::MessageSender,
    coordinator_service_error::{CoordinatorError, CoordinatorResult},
    ProverInfo,
};

pub struct ProversPool {
    /// Map of prover_id to ProverConnection
    provers: RwLock<HashMap<ProverId, ProverInfo>>,
}

impl ProversPool {
    /// Create a new ProversPool
    pub fn new() -> Self {
        Self { provers: RwLock::new(HashMap::new()) }
    }

    pub async fn num_provers(&self) -> usize {
        self.provers.read().await.len()
    }

    pub async fn idle_provers(&self) -> usize {
        self.provers.read().await.values().filter(|p| p.state == ProverState::Idle).count()
    }

    pub async fn busy_provers(&self) -> usize {
        self.provers
            .read()
            .await
            .values()
            .filter(|p| matches!(p.state, ProverState::Computing(_)))
            .count()
    }

    pub async fn compute_capacity(&self) -> ComputeCapacity {
        let total_capacity: u32 =
            self.provers.read().await.values().map(|p| p.compute_capacity.compute_units).sum();

        ComputeCapacity::from(total_capacity)
    }

    pub async fn provers_list(&self) -> ProversListDto {
        let provers = self
            .provers
            .read()
            .await
            .values()
            .map(|prover_info| ProverInfoDto {
                prover_id: prover_info.prover_id.clone(),
                state: prover_info.state.clone(),
                compute_capacity: prover_info.compute_capacity,
                connected_at: prover_info.connected_at,
                last_heartbeat: prover_info.last_heartbeat,
            })
            .collect();

        ProversListDto { provers }
    }

    /// Register a new prover
    pub async fn register_prover(
        &self,
        prover_id: ProverId,
        compute_capacity: impl Into<ComputeCapacity>,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> CoordinatorResult<()> {
        let connection = ProverInfo::new(prover_id.clone(), compute_capacity.into(), msg_sender);

        // Check if prover_id is already registered
        if self.provers.read().await.contains_key(&prover_id) {
            let msg = format!("Prover {} is already registered", prover_id);
            warn!("{}", msg);
            Err(CoordinatorError::InvalidRequest(msg))
        } else {
            self.provers.write().await.insert(prover_id.clone(), connection);
            info!("Registered prover: {} (total: {})", prover_id, self.num_provers().await);
            Ok(())
        }
    }

    pub async fn reconnect_prover(
        &self,
        prover_id: ProverId,
        compute_capacity: impl Into<ComputeCapacity>,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> CoordinatorResult<()> {
        match self.provers.write().await.get_mut(&prover_id) {
            Some(existing_prover) => {
                existing_prover.state = ProverState::Idle;
                existing_prover.compute_capacity = compute_capacity.into();
                existing_prover.msg_sender = msg_sender;
                existing_prover.update_last_heartbeat();

                info!("Reconnected prover: {} (total: {})", prover_id, self.num_provers().await);
                Ok(())
            }
            None => {
                let msg =
                    format!("Prover ID {} is not registered. Impossible to reconnect.", prover_id);
                warn!("{}", msg);
                Err(CoordinatorError::InvalidRequest(msg))
            }
        }
    }

    /// Unregister a prover
    pub async fn unregister_prover(&self, prover_id: &ProverId) -> CoordinatorResult<()> {
        self.provers.write().await.remove(prover_id).map(|_| ()).ok_or_else(|| {
            let msg = format!("Prover {prover_id} not found for removal");
            warn!("{}", msg);
            CoordinatorError::NotFoundOrInaccessible
        })
    }

    pub async fn prover_state(&self, prover_id: &ProverId) -> Option<ProverState> {
        self.provers.read().await.get(prover_id).map(|p| p.state.clone())
    }

    pub async fn mark_provers_with_state(
        &self,
        prover_ids: &[ProverId],
        state: ProverState,
    ) -> CoordinatorResult<()> {
        for prover_id in prover_ids {
            self.mark_prover_with_state(prover_id, state.clone()).await?;
        }
        Ok(())
    }

    pub async fn mark_prover_with_state(
        &self,
        prover_id: &ProverId,
        state: ProverState,
    ) -> CoordinatorResult<()> {
        if let Some(prover) = self.provers.write().await.get_mut(prover_id) {
            prover.state = state;
            Ok(())
        } else {
            Err(CoordinatorError::NotFoundOrInaccessible)
        }
    }

    pub async fn send_message(
        &self,
        prover_id: &ProverId,
        message: CoordinatorMessageDto,
    ) -> CoordinatorResult<()> {
        if let Some(prover) = self.provers.read().await.get(prover_id) {
            prover.msg_sender.send(message).map_err(|e| {
                let msg = format!("Failed to send message to prover {prover_id}: {}", e);
                warn!("{}", msg);
                CoordinatorError::Internal(msg)
            })
        } else {
            let msg = format!("Prover {prover_id} not found for sending message");
            warn!("{}", msg);
            Err(CoordinatorError::NotFoundOrInaccessible)
        }
    }

    pub async fn update_last_heartbeat(&self, prover_id: &ProverId) -> CoordinatorResult<()> {
        if let Some(prover) = self.provers.write().await.get_mut(prover_id) {
            prover.update_last_heartbeat();
            Ok(())
        } else {
            let msg = format!("Prover {prover_id} not found for heartbeat update");
            warn!("{}", msg);
            Err(CoordinatorError::NotFoundOrInaccessible)
        }
    }

    pub async fn partition_and_allocate_by_capacity(
        &self,
        required_compute_capacity: ComputeCapacity,
        execution_mode: JobExecutionMode,
    ) -> CoordinatorResult<(Vec<ProverId>, Vec<Vec<u32>>)> {
        if execution_mode.is_simulating() && self.num_provers().await != 1 {
            warn!("Simulated mode enabled but there are multiple provers connected. Only the first prover will be used.");
            return Err(CoordinatorError::InvalidRequest(
                "Simulated mode can only be used when there is exactly one prover connected"
                    .to_string(),
            ));
        }

        if required_compute_capacity.compute_units == 0 {
            return Err(CoordinatorError::InvalidArgument(
                "Compute capacity must be greater than 0".to_string(),
            ));
        }

        let provers = self.provers.write().await;

        let available_provers: Vec<(&ProverId, &ProverInfo)> = if execution_mode.is_simulating() {
            // Copy the only available idle prover 'times' times
            if let Some((prover_id, prover_info)) =
                provers.iter().find(|(_, p)| matches!(p.state, ProverState::Idle))
            {
                let times = (required_compute_capacity.compute_units as f32
                    / prover_info.compute_capacity.compute_units as f32)
                    .ceil() as u32;

                vec![(prover_id, prover_info); times as usize]
            } else {
                return Err(CoordinatorError::InsufficientCapacity);
            }
        } else {
            provers.iter().filter(|(_, p)| matches!(p.state, ProverState::Idle)).collect()
        };

        let available_capacity: u32 =
            available_provers.iter().map(|(_, p)| p.compute_capacity.compute_units).sum();

        if required_compute_capacity.compute_units > available_capacity {
            return Err(CoordinatorError::InsufficientCapacity);
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
}
