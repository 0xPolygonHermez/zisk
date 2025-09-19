//! # Provers Pool Management
//!
//! Manages the pool of connected provers, their states, and capacity allocation
//! for distributed proof generation jobs.

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

/// Manages connected provers and their resource allocation.
///
/// Handles prover registration, state management, message routing, and
/// capacity-based work allocation across the distributed prover network.
pub struct ProversPool {
    /// Map of prover_id to ProverConnection
    provers: RwLock<HashMap<ProverId, ProverInfo>>,
}

impl ProversPool {
    /// Creates a new empty provers pool.
    pub fn new() -> Self {
        Self { provers: RwLock::new(HashMap::new()) }
    }

    /// Returns the total number of registered provers.
    pub async fn num_provers(&self) -> usize {
        self.provers.read().await.len()
    }

    /// Returns the number of provers currently available for new jobs.
    pub async fn idle_provers(&self) -> usize {
        self.provers.read().await.values().filter(|p| p.state == ProverState::Idle).count()
    }

    /// Returns the number of provers currently executing tasks.
    pub async fn busy_provers(&self) -> usize {
        self.provers
            .read()
            .await
            .values()
            .filter(|p| matches!(p.state, ProverState::Computing(_)))
            .count()
    }

    /// Calculates total compute capacity across all registered provers.
    pub async fn compute_capacity(&self) -> ComputeCapacity {
        let total_capacity: u32 =
            self.provers.read().await.values().map(|p| p.compute_capacity.compute_units).sum();

        ComputeCapacity::from(total_capacity)
    }

    /// Returns detailed information about all registered provers.
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

    /// Registers a new prover with the pool.
    ///
    /// # Parameters
    ///
    /// - `prover_id`: Unique identifier for the prover.
    /// - `compute_capacity`: The compute capacity of the prover.
    /// - `msg_sender`: Channel to send messages to the prover.
    ///
    /// # Returns
    ///
    /// `InvalidRequest` error if prover ID is already registered.
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

    /// Reconnects an existing prover with updated connection details.
    ///
    /// Resets prover state to Idle and updates capacity and message channel.
    ///
    /// # Parameters
    ///
    /// - `prover_id`: Unique identifier for the prover.
    /// - `compute_capacity`: The new compute capacity of the prover.
    /// - `msg_sender`: New channel to send messages to the prover.
    ///
    /// # Returns
    ///
    /// `InvalidRequest` error if prover ID is not registered.
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

    /// Removes a prover from the pool.
    ///
    /// # Parameters
    ///
    /// - `prover_id`: Unique identifier for the prover to be removed.
    pub async fn unregister_prover(&self, prover_id: &ProverId) -> CoordinatorResult<()> {
        self.provers.write().await.remove(prover_id).map(|_| ()).ok_or_else(|| {
            let msg = format!("Prover {prover_id} not found for removal");
            warn!("{}", msg);
            CoordinatorError::NotFoundOrInaccessible
        })
    }

    /// Gets the current state of a specific prover.
    ///
    /// # Parameters
    ///
    /// - `prover_id`: Unique identifier for the prover.
    pub async fn prover_state(&self, prover_id: &ProverId) -> Option<ProverState> {
        self.provers.read().await.get(prover_id).map(|p| p.state.clone())
    }

    /// Updates the state for multiple provers atomically.
    ///
    /// # Parameters
    ///
    /// - `prover_ids`: List of prover IDs to update.
    /// - `state`: New state to set for the specified provers.
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

    /// Updates the state of a single prover.
    ///
    /// # Parameters
    ///
    /// - `prover_id`: Unique identifier for the prover.
    /// - `state`: New state to set for the prover.
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

    /// Sends a message to a specific prover.
    ///
    /// # Parameters
    ///
    /// - `prover_id`: Unique identifier for the prover.
    /// - `message`: The message to send to the prover.
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

    /// Updates the last heartbeat timestamp for a prover.
    ///
    /// # Parameters
    ///
    /// - `prover_id`: Unique identifier for the prover.
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

    /// Selects provers and allocates compute units based on required capacity.
    ///
    /// Uses round-robin allocation to distribute work units across selected provers
    /// while respecting individual prover capacity limits.
    ///
    /// # Parameters
    ///
    /// - `required_compute_capacity`: Total compute capacity needed for the job.
    /// - `execution_mode`: Job execution mode (standard or simulation).
    ///
    /// # Returns
    /// Selected prover IDs and their allocated compute unit assignments
    pub async fn partition_and_allocate_by_capacity(
        &self,
        required_compute_capacity: ComputeCapacity,
        execution_mode: JobExecutionMode,
    ) -> CoordinatorResult<(Vec<ProverId>, Vec<Vec<u32>>)> {
        // Simulation mode requires exactly one prover
        if execution_mode.is_simulating() && self.num_provers().await != 1 {
            warn!("Simulated mode enabled but there are multiple provers connected. Only the first prover will be used.");
            return Err(CoordinatorError::InvalidRequest(
                "Simulated mode can only be used when there is exactly one prover connected"
                    .to_string(),
            ));
        }

        // Validate required capacity, must be greater than 0
        if required_compute_capacity.compute_units == 0 {
            return Err(CoordinatorError::InvalidArgument(
                "Compute capacity must be greater than 0".to_string(),
            ));
        }

        let provers = self.provers.write().await;

        // For simulation mode, replicate single prover multiple times
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
            // Standard mode: use all idle provers
            provers.iter().filter(|(_, p)| matches!(p.state, ProverState::Idle)).collect()
        };

        let available_capacity: u32 =
            available_provers.iter().map(|(_, p)| p.compute_capacity.compute_units).sum();

        // Check if we have enough total capacity
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

        // Step 2: Distribute work units using round-robin allocation
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
