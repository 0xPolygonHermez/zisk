//! # Workers Pool Management
//!
//! Manages the pool of connected workers, their states, and capacity allocation
//! for distributed proof generation jobs.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt::Display;
use tokio::sync::RwLock;
use tracing::{info, warn};
use zisk_distributed_common::{
    ComputeCapacity, CoordinatorMessageDto, JobExecutionMode, WorkerId, WorkerInfoDto, WorkerState,
    WorkersListDto,
};

use crate::{
    coordinator::MessageSender,
    coordinator_errors::{CoordinatorError, CoordinatorResult},
};

/// Information about a connected worker
pub struct WorkerInfo {
    pub worker_id: WorkerId,
    pub state: WorkerState,
    pub compute_capacity: ComputeCapacity,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub msg_sender: Box<dyn MessageSender + Send + Sync>,
}

impl WorkerInfo {
    pub fn new(
        worker_id: WorkerId,
        compute_capacity: ComputeCapacity,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> Self {
        let now = Utc::now();
        Self {
            worker_id,
            state: WorkerState::Idle,
            compute_capacity,
            connected_at: now,
            last_heartbeat: now,
            msg_sender,
        }
    }

    pub fn update_last_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }
}

impl Display for WorkerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WorkerInfo(state: {}, capacity: {}, connected_at: {}, last_heartbeat: {})",
            self.state, self.compute_capacity, self.connected_at, self.last_heartbeat
        )
    }
}

/// Manages connected workers and their resource allocation.
///
/// Handles worker registration, state management, message routing, and
/// capacity-based work allocation across the ZisK distributed network.
pub struct WorkersPool {
    /// Map of worker_id to WorkerInfo
    workers: RwLock<HashMap<WorkerId, WorkerInfo>>,
}

impl WorkersPool {
    /// Creates a new empty workers pool.
    pub fn new() -> Self {
        Self { workers: RwLock::new(HashMap::new()) }
    }

    /// Returns the total number of registered workers.
    pub async fn num_workers(&self) -> usize {
        self.workers.read().await.len()
    }

    /// Returns the number of workers currently available for new jobs.
    pub async fn idle_workers(&self) -> usize {
        self.workers.read().await.values().filter(|p| p.state == WorkerState::Idle).count()
    }

    /// Returns the number of workers currently executing tasks.
    pub async fn busy_workers(&self) -> usize {
        self.workers
            .read()
            .await
            .values()
            .filter(|p| matches!(p.state, WorkerState::Computing(_)))
            .count()
    }

    /// Calculates total compute capacity across all registered workers.
    pub async fn compute_capacity(&self) -> ComputeCapacity {
        let total_capacity: u32 =
            self.workers.read().await.values().map(|p| p.compute_capacity.compute_units).sum();

        ComputeCapacity::from(total_capacity)
    }

    /// Returns detailed information about all registered workers.
    pub async fn workers_list(&self) -> WorkersListDto {
        let workers = self
            .workers
            .read()
            .await
            .values()
            .map(|worker_info| WorkerInfoDto {
                worker_id: worker_info.worker_id.clone(),
                state: worker_info.state.clone(),
                compute_capacity: worker_info.compute_capacity,
                connected_at: worker_info.connected_at,
                last_heartbeat: worker_info.last_heartbeat,
            })
            .collect();

        WorkersListDto { workers }
    }

    /// Registers a new worker with the pool.
    ///
    /// # Parameters
    ///
    /// - `worker_id`: Unique identifier for the worker.
    /// - `compute_capacity`: The compute capacity of the worker.
    /// - `msg_sender`: Channel to send messages to the worker.
    ///
    /// # Returns
    ///
    /// `InvalidRequest` error if worker ID is already registered.
    pub async fn register_worker(
        &self,
        worker_id: WorkerId,
        compute_capacity: impl Into<ComputeCapacity>,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> CoordinatorResult<()> {
        let connection = WorkerInfo::new(worker_id.clone(), compute_capacity.into(), msg_sender);

        // Check if worker_id is already registered
        if self.workers.read().await.contains_key(&worker_id) {
            let msg = format!("Worker {} is already registered", worker_id);
            warn!("{}", msg);
            Err(CoordinatorError::InvalidRequest(msg))
        } else {
            self.workers.write().await.insert(worker_id.clone(), connection);
            info!("Registered worker: {} (total: {})", worker_id, self.num_workers().await);
            Ok(())
        }
    }

    /// Reconnects an existing worker with updated connection details.
    ///
    /// Resets worker state to Idle and updates capacity and message channel.
    ///
    /// # Parameters
    ///
    /// - `worker_id`: Unique identifier for the worker.
    /// - `compute_capacity`: The new compute capacity of the worker.
    /// - `msg_sender`: New channel to send messages to the worker.
    ///
    /// # Returns
    ///
    /// `InvalidRequest` error if worker ID is not registered.
    pub async fn reconnect_worker(
        &self,
        worker_id: WorkerId,
        compute_capacity: impl Into<ComputeCapacity>,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> CoordinatorResult<()> {
        match self.workers.write().await.get_mut(&worker_id) {
            Some(existing_worker) => {
                existing_worker.state = WorkerState::Idle;
                existing_worker.compute_capacity = compute_capacity.into();
                existing_worker.msg_sender = msg_sender;
                existing_worker.update_last_heartbeat();

                info!("Reconnected worker: {} (total: {})", worker_id, self.num_workers().await);
                Ok(())
            }
            None => {
                let msg =
                    format!("Worker ID {} is not registered. Impossible to reconnect.", worker_id);
                warn!("{}", msg);
                Err(CoordinatorError::InvalidRequest(msg))
            }
        }
    }

    /// Removes a worker from the pool.
    ///
    /// # Parameters
    ///
    /// - `worker_id`: Unique identifier for the worker to be removed.
    pub async fn unregister_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        let mut workers = self.workers.write().await;
        match workers.remove(worker_id) {
            Some(_) => {
                let total = workers.len(); // Get count from the current HashMap
                drop(workers); // Release the lock before logging
                info!("Unregistered worker: {} (total: {})", worker_id, total);
                Ok(())
            }
            None => {
                let msg = format!("Worker {worker_id} not found for removal");
                warn!("{}", msg);
                Err(CoordinatorError::NotFoundOrInaccessible)
            }
        }
    }

    /// Gets the current state of a specific worker.
    ///
    /// # Parameters
    ///
    /// - `worker_id`: Unique identifier for the worker.
    pub async fn worker_state(&self, worker_id: &WorkerId) -> Option<WorkerState> {
        self.workers.read().await.get(worker_id).map(|p| p.state.clone())
    }

    /// Updates the state for multiple workers atomically.
    ///
    /// # Parameters
    ///
    /// - `worker_ids`: List of worker IDs to update.
    /// - `state`: New state to set for the specified workers.
    pub async fn mark_workers_with_state(
        &self,
        worker_ids: &[WorkerId],
        state: WorkerState,
    ) -> CoordinatorResult<()> {
        for worker_id in worker_ids {
            self.mark_worker_with_state(worker_id, state.clone()).await?;
        }
        Ok(())
    }

    /// Updates the state of a single worker.
    ///
    /// # Parameters
    ///
    /// - `worker_id`: Unique identifier for the worker.
    /// - `state`: New state to set for the worker.
    pub async fn mark_worker_with_state(
        &self,
        worker_id: &WorkerId,
        state: WorkerState,
    ) -> CoordinatorResult<()> {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.state = state;
            Ok(())
        } else {
            Err(CoordinatorError::NotFoundOrInaccessible)
        }
    }

    /// Sends a message to a specific worker.
    ///
    /// # Parameters
    ///
    /// - `worker_id`: Unique identifier for the worker.
    /// - `message`: The message to send to the worker.
    pub async fn send_message(
        &self,
        worker_id: &WorkerId,
        message: CoordinatorMessageDto,
    ) -> CoordinatorResult<()> {
        if let Some(worker) = self.workers.read().await.get(worker_id) {
            worker.msg_sender.send(message).map_err(|e| {
                let msg = format!("Failed to send message to worker {worker_id}: {}", e);
                warn!("{}", msg);
                CoordinatorError::Internal(msg)
            })
        } else {
            let msg = format!("Worker {worker_id} not found for sending message");
            warn!("{}", msg);
            Err(CoordinatorError::NotFoundOrInaccessible)
        }
    }

    /// Updates the last heartbeat timestamp for a worker.
    ///
    /// # Parameters
    ///
    /// - `worker_id`: Unique identifier for the worker.
    pub async fn update_last_heartbeat(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.update_last_heartbeat();
            Ok(())
        } else {
            let msg = format!("Worker {worker_id} not found for heartbeat update");
            warn!("{}", msg);
            Err(CoordinatorError::NotFoundOrInaccessible)
        }
    }

    /// Selects workers and allocates compute units based on required capacity.
    ///
    /// Uses round-robin allocation to distribute work units across selected workers
    /// while respecting individual worker capacity limits.
    ///
    /// # Parameters
    ///
    /// - `required_compute_capacity`: Total compute capacity needed for the job.
    /// - `execution_mode`: Job execution mode (standard or simulation).
    ///
    /// # Returns
    /// Selected worker IDs and their allocated compute unit assignments
    pub async fn partition_and_allocate_by_capacity(
        &self,
        required_compute_capacity: ComputeCapacity,
        execution_mode: JobExecutionMode,
    ) -> CoordinatorResult<(Vec<WorkerId>, Vec<Vec<u32>>)> {
        // Simulation mode requires exactly one worker
        if execution_mode.is_simulating() && self.num_workers().await != 1 {
            warn!("Simulated mode enabled but there are multiple workers connected. Only the first worker will be used.");
            return Err(CoordinatorError::InvalidRequest(
                "Simulated mode can only be used when there is exactly one worker connected"
                    .to_string(),
            ));
        }

        // Validate required capacity, must be greater than 0
        if required_compute_capacity.compute_units == 0 {
            return Err(CoordinatorError::InvalidArgument(
                "Compute capacity must be greater than 0".to_string(),
            ));
        }

        let workers = self.workers.write().await;

        // For simulation mode, replicate single worker multiple times
        let available_workers: Vec<(&WorkerId, &WorkerInfo)> = if execution_mode.is_simulating() {
            // Copy the only available idle worker 'times' times
            if let Some((worker_id, worker_info)) =
                workers.iter().find(|(_, p)| matches!(p.state, WorkerState::Idle))
            {
                let times = (required_compute_capacity.compute_units as f32
                    / worker_info.compute_capacity.compute_units as f32)
                    .ceil() as u32;

                vec![(worker_id, worker_info); times as usize]
            } else {
                return Err(CoordinatorError::InsufficientCapacity);
            }
        } else {
            // Standard mode: use all idle workers
            workers.iter().filter(|(_, p)| matches!(p.state, WorkerState::Idle)).collect()
        };

        let available_capacity: u32 =
            available_workers.iter().map(|(_, p)| p.compute_capacity.compute_units).sum();

        // Check if we have enough total capacity
        if required_compute_capacity.compute_units > available_capacity {
            return Err(CoordinatorError::InsufficientCapacity);
        }

        let mut selected_workers = Vec::new();
        let mut worker_capacities = Vec::new();
        let mut total_capacity = 0;

        // Step 1: Select workers that can cover the required compute capacity
        for (worker_id, worker_info) in available_workers {
            if matches!(worker_info.state, WorkerState::Idle) {
                selected_workers.push(worker_id.clone());
                worker_capacities.push(worker_info.compute_capacity.compute_units);
                total_capacity += worker_info.compute_capacity.compute_units;

                // Stop when we have enough capacity
                if total_capacity >= required_compute_capacity.compute_units {
                    break;
                }
            }
        }

        drop(workers);

        // Step 2: Distribute work units using round-robin allocation
        let num_workers = selected_workers.len();
        let total_units = required_compute_capacity.compute_units;
        let mut worker_allocations = vec![Vec::new(); num_workers];

        // Round-robin assignment of compute units
        for unit in 0..total_units {
            let worker_idx = (unit as usize) % num_workers;

            // Check if this worker still has capacity
            if worker_allocations[worker_idx].len() < worker_capacities[worker_idx] as usize {
                worker_allocations[worker_idx].push(unit);
            } else {
                // If this worker is at capacity, find the next available worker
                let mut found = false;
                for offset in 1..num_workers {
                    let next_idx = (worker_idx + offset) % num_workers;
                    if worker_allocations[next_idx].len() < worker_capacities[next_idx] as usize {
                        worker_allocations[next_idx].push(unit);
                        found = true;
                        break;
                    }
                }

                if !found {
                    warn!("Could not assign compute unit {} to any worker", unit);
                    break;
                }
            }
        }

        Ok((selected_workers, worker_allocations))
    }
}
