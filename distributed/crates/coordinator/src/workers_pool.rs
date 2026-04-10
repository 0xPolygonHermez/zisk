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
    ComputeCapacity, CoordinatorMessageDto, JobExecutionMode, JobId, JobPhase, WorkerId,
    WorkerInfoDto, WorkerState, WorkersListDto,
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
    pub connection_generation: u64,
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
            connection_generation: 0,
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

impl Default for WorkersPool {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkersPool {
    /// Creates a new empty workers pool.
    pub fn new() -> Self {
        Self { workers: RwLock::new(HashMap::new()) }
    }

    /// Returns the worker's state and connection generation if present.
    pub async fn worker_state_and_generation(
        &self,
        worker_id: &WorkerId,
    ) -> Option<(WorkerState, u64)> {
        let workers = self.workers.read().await;
        workers.get(worker_id).map(|w| (w.state.clone(), w.connection_generation))
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
        let total_capacity: u32 = self
            .workers
            .read()
            .await
            .values()
            .map(|p| {
                if p.state != WorkerState::Disconnected {
                    p.compute_capacity.compute_units
                } else {
                    0
                }
            })
            .sum();

        ComputeCapacity::from(total_capacity)
    }

    /// Calculates total available compute capacity across all registered workers.
    pub async fn available_compute_capacity(&self) -> ComputeCapacity {
        let total_capacity: u32 = self
            .workers
            .read()
            .await
            .values()
            .filter(|p| p.state == WorkerState::Idle)
            .map(|p| p.compute_capacity.compute_units)
            .sum();

        ComputeCapacity::from(total_capacity)
    }

    /// Returns (num_workers, compute_capacity, available_compute_capacity) under a single read lock.
    pub async fn pool_stats(&self) -> (usize, ComputeCapacity, ComputeCapacity) {
        let workers = self.workers.read().await;
        let mut total = 0;
        let mut cc: u32 = 0;
        let mut acc: u32 = 0;
        for w in workers.values() {
            if w.state != WorkerState::Disconnected {
                total += 1;
                cc += w.compute_capacity.compute_units;
            }
            if w.state == WorkerState::Idle {
                acc += w.compute_capacity.compute_units;
            }
        }
        (total, ComputeCapacity::from(cc), ComputeCapacity::from(acc))
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
        let mut workers = self.workers.write().await;

        let is_new_worker = if let Some(worker) = workers.get_mut(&worker_id) {
            if worker.state != WorkerState::Disconnected && worker.state != WorkerState::Idle {
                let msg =
                    format!("Worker {} is already connected (state: {})", worker_id, worker.state);
                warn!("{}", msg);
                return Err(CoordinatorError::InvalidRequest(msg));
            } else {
                worker.state = WorkerState::Idle;
                worker.compute_capacity = connection.compute_capacity;
                worker.msg_sender = connection.msg_sender;
                worker.connection_generation += 1;
                worker.update_last_heartbeat();

                false
            }
        } else {
            workers.insert(worker_id.clone(), connection);

            true
        };

        drop(workers);

        let action = if is_new_worker { "Registered" } else { "Reconnected" };
        let (total, cc, acc) = self.pool_stats().await;
        info!("{} worker: {} (total: {} CC: {} ACC: {})", action, worker_id, total, cc, acc);

        Ok(())
    }

    // /// Reconnects an existing worker with updated connection details.
    // ///
    // /// Resets worker state to Idle and updates capacity and message channel.
    // ///
    // /// # Parameters
    // ///
    // /// - `worker_id`: Unique identifier for the worker.
    // /// - `compute_capacity`: The new compute capacity of the worker.
    // /// - `msg_sender`: New channel to send messages to the worker.
    // ///
    // /// # Returns
    // ///
    // /// `InvalidRequest` error if worker ID is not registered.
    // pub async fn reconnect_worker(
    //     &self,
    //     worker_id: WorkerId,
    //     compute_capacity: impl Into<ComputeCapacity>,
    //     msg_sender: Box<dyn MessageSender + Send + Sync>,
    // ) -> CoordinatorResult<()> {
    //     let mut workers = self.workers.write().await;

    //     if let Some(worker) = workers.get_mut(&worker_id) {
    //         if worker.state != WorkerState::Disconnected {
    //             let msg = format!("Worker {} is already registered", worker_id);
    //             warn!("{}", msg);
    //             return Err(CoordinatorError::InvalidRequest(msg));
    //         } else {
    //             worker.state = WorkerState::Idle;
    //             worker.compute_capacity = compute_capacity.into();
    //             worker.msg_sender = msg_sender;
    //             worker.update_last_heartbeat();
    //         }

    //         drop(workers);

    //         info!(
    //             "Reconnected worker: {} (total: {} CC: {} ACC: {})",
    //             worker_id,
    //             self.num_workers().await,
    //             self.compute_capacity().await,
    //             self.available_compute_capacity().await
    //         );
    //     } else {
    //         let connection =
    //             WorkerInfo::new(worker_id.clone(), compute_capacity.into(), msg_sender);
    //         workers.insert(worker_id.clone(), connection);

    //         drop(workers);

    //         info!(
    //             "Registered worker: {} (total: {} CC: {} ACC: {})",
    //             worker_id,
    //             self.num_workers().await,
    //             self.compute_capacity().await,
    //             self.available_compute_capacity().await
    //         );
    //     }

    //     Ok(())
    // }

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
                info!(
                    "Unregistered worker: {} (total: {} CC: {} ACC: {})",
                    worker_id,
                    total,
                    self.compute_capacity().await,
                    self.available_compute_capacity().await
                );

                Ok(())
            }
            None => {
                let msg = format!("Worker {worker_id} not found for removal");
                warn!("{}", msg);
                Err(CoordinatorError::NotFoundOrInaccessible)
            }
        }
    }

    pub async fn disconnect_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        let mut workers = self.workers.write().await;
        let result = Self::do_disconnect(&mut workers, worker_id, None);
        drop(workers);
        if result? {
            let (total, cc, acc) = self.pool_stats().await;
            info!("Disconnected worker: {} (total: {} CC: {} ACC: {})", worker_id, total, cc, acc);
        }
        Ok(())
    }

    /// Disconnects a worker only if its connection generation matches.
    /// Returns Ok(()) silently if generation doesn't match (stale guard)
    /// or if the worker is already disconnected (idempotent).
    pub async fn disconnect_worker_if_generation(
        &self,
        worker_id: &WorkerId,
        expected_generation: u64,
    ) -> CoordinatorResult<()> {
        let mut workers = self.workers.write().await;
        let result = Self::do_disconnect(&mut workers, worker_id, Some(expected_generation));
        drop(workers);
        if result? {
            let (total, cc, acc) = self.pool_stats().await;
            info!("Disconnected worker: {} (total: {} CC: {} ACC: {})", worker_id, total, cc, acc);
        }
        Ok(())
    }

    /// Core disconnect logic. Returns `Ok(true)` if the worker was disconnected,
    /// `Ok(false)` if it was a no-op (stale generation or already disconnected).
    fn do_disconnect(
        workers: &mut HashMap<WorkerId, WorkerInfo>,
        worker_id: &WorkerId,
        expected_generation: Option<u64>,
    ) -> CoordinatorResult<bool> {
        match workers.get_mut(worker_id) {
            Some(worker)
                if expected_generation.is_some_and(|g| g != worker.connection_generation) =>
            {
                Ok(false)
            }
            Some(worker) if worker.state == WorkerState::Disconnected => Ok(false),
            Some(worker) => {
                worker.state = WorkerState::Disconnected;
                Ok(true)
            }
            None => {
                let msg =
                    format!("Worker ID {} is not registered. Impossible to disconnect.", worker_id);
                warn!("{}", msg);
                Err(CoordinatorError::InvalidRequest(msg))
            }
        }
    }

    /// Returns the connection generation for a worker.
    pub async fn connection_generation(&self, worker_id: &WorkerId) -> Option<u64> {
        self.workers.read().await.get(worker_id).map(|w| w.connection_generation)
    }

    /// Removes worker entries that have been Disconnected for longer than a threshold.
    /// Prevents unbounded growth of the workers HashMap.
    pub async fn remove_stale_disconnected(&self, threshold: chrono::Duration) {
        let now = Utc::now();
        let mut workers = self.workers.write().await;
        workers.retain(|id, w| {
            if w.state == WorkerState::Disconnected {
                let elapsed = now.signed_duration_since(w.last_heartbeat);
                if elapsed >= threshold {
                    info!("Removing stale disconnected worker: {}", id);
                    return false;
                }
            }
            true
        });
    }

    /// Sets a worker's last heartbeat to a specific time. Used for testing only.
    pub async fn set_last_heartbeat(
        &self,
        worker_id: &WorkerId,
        time: DateTime<Utc>,
    ) -> CoordinatorResult<()> {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.last_heartbeat = time;
            Ok(())
        } else {
            Err(CoordinatorError::NotFoundOrInaccessible)
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

    /// Marks any `Computing` workers in the given list as `Idle` under a single lock.
    /// Workers that are not `Computing` or not found are silently skipped.
    pub async fn mark_computing_workers_idle(&self, worker_ids: &[WorkerId]) {
        let mut workers = self.workers.write().await;
        let mut transitioned = Vec::new();
        for wid in worker_ids {
            if let Some(worker) = workers.get_mut(wid) {
                if matches!(worker.state, WorkerState::Computing(_)) {
                    worker.state = WorkerState::Idle;
                    transitioned.push(wid.clone());
                }
            }
        }
        drop(workers);
        if !transitioned.is_empty() {
            let (total, cc, acc) = self.pool_stats().await;
            for wid in &transitioned {
                info!("Worker {} marked idle (total: {} CC: {} ACC: {})", wid, total, cc, acc);
            }
        }
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
            worker.state = state.clone();
        } else {
            return Err(CoordinatorError::NotFoundOrInaccessible);
        }

        if state == WorkerState::Idle {
            let (total, cc, acc) = self.pool_stats().await;
            info!("Worker {} available (total: {} CC: {} ACC: {})", worker_id, total, cc, acc);
        }
        Ok(())
    }

    /// Returns computing workers whose last heartbeat is older than the given threshold.
    pub async fn get_stale_computing_workers(
        &self,
        threshold: chrono::Duration,
    ) -> Vec<(WorkerId, JobId, JobPhase)> {
        let now = Utc::now();
        let workers = self.workers.read().await;

        workers
            .values()
            .filter_map(|w| {
                if let WorkerState::Computing((job_id, phase)) = &w.state {
                    let elapsed = now.signed_duration_since(w.last_heartbeat);
                    if elapsed >= threshold {
                        return Some((w.worker_id.clone(), job_id.clone(), phase.clone()));
                    }
                }
                None
            })
            .collect()
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
        minimal_compute_capacity: ComputeCapacity,
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

        if minimal_compute_capacity.compute_units > required_compute_capacity.compute_units {
            return Err(CoordinatorError::InvalidArgument(
                "Minimal compute capacity cannot exceed required capacity".to_string(),
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
        if minimal_compute_capacity.compute_units > available_capacity {
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
        let mut worker_allocations = vec![Vec::new(); num_workers];

        // Round-robin assignment of compute units
        for unit in 0..total_capacity {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use zisk_distributed_common::WorkerState;

    #[tokio::test]
    async fn test_disconnect_idempotent() {
        let pool = WorkersPool::new();
        let (worker_id, _msgs) = register_test_worker(&pool, "w1").await;

        // First disconnect succeeds
        pool.disconnect_worker(&worker_id).await.unwrap();
        assert_eq!(pool.worker_state(&worker_id).await, Some(WorkerState::Disconnected));

        // Second disconnect also succeeds (idempotent)
        pool.disconnect_worker(&worker_id).await.unwrap();
        assert_eq!(pool.worker_state(&worker_id).await, Some(WorkerState::Disconnected));
    }

    #[tokio::test]
    async fn test_get_stale_computing_workers() {
        let pool = WorkersPool::new();
        let (w1, _) = register_test_worker(&pool, "w1").await;
        let (w2, _) = register_test_worker(&pool, "w2").await;
        let (w3, _) = register_test_worker(&pool, "w3").await;

        let job_id = JobId::from("job-1".to_string());

        // Mark all three as Computing
        pool.mark_worker_with_state(
            &w1,
            WorkerState::Computing((job_id.clone(), JobPhase::Contributions)),
        )
        .await
        .unwrap();
        pool.mark_worker_with_state(
            &w2,
            WorkerState::Computing((job_id.clone(), JobPhase::Contributions)),
        )
        .await
        .unwrap();
        pool.mark_worker_with_state(
            &w3,
            WorkerState::Computing((job_id.clone(), JobPhase::Contributions)),
        )
        .await
        .unwrap();

        // Set w1 and w2 heartbeats to 100 seconds ago (stale with 30s interval × 3 missed = 90s)
        {
            let mut workers = pool.workers.write().await;
            let old_time = Utc::now() - chrono::Duration::seconds(100);
            workers.get_mut(&w1).unwrap().last_heartbeat = old_time;
            workers.get_mut(&w2).unwrap().last_heartbeat = old_time;
            // w3 heartbeat stays fresh
        }

        let stale = pool.get_stale_computing_workers(chrono::Duration::seconds(90)).await;
        let stale_ids: Vec<&WorkerId> = stale.iter().map(|(id, _, _)| id).collect();

        assert_eq!(stale.len(), 2);
        assert!(stale_ids.contains(&&w1));
        assert!(stale_ids.contains(&&w2));
    }

    #[tokio::test]
    async fn test_get_stale_workers_ignores_idle() {
        let pool = WorkersPool::new();
        let (w1, _) = register_test_worker(&pool, "w1").await;

        // w1 is Idle with an old heartbeat — should NOT be returned
        {
            let mut workers = pool.workers.write().await;
            workers.get_mut(&w1).unwrap().last_heartbeat =
                Utc::now() - chrono::Duration::seconds(200);
        }

        let stale = pool.get_stale_computing_workers(chrono::Duration::seconds(90)).await;
        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn test_disconnect_if_generation_stale() {
        let pool = WorkersPool::new();
        let (w1, _) = register_test_worker(&pool, "w1").await;
        assert_eq!(pool.connection_generation(&w1).await, Some(0));

        // Disconnect then re-register (simulates reconnection → gen becomes 1)
        pool.disconnect_worker(&w1).await.unwrap();
        let (sender, _) = MockMessageSender::new();
        pool.register_worker(w1.clone(), 1u32, Box::new(sender)).await.unwrap();
        assert_eq!(pool.connection_generation(&w1).await, Some(1));

        // Stale guard with gen 0 should be a no-op
        pool.disconnect_worker_if_generation(&w1, 0).await.unwrap();
        assert_eq!(pool.worker_state(&w1).await, Some(WorkerState::Idle));
    }

    #[tokio::test]
    async fn test_disconnect_if_generation_current() {
        let pool = WorkersPool::new();
        let (w1, _) = register_test_worker(&pool, "w1").await;
        assert_eq!(pool.connection_generation(&w1).await, Some(0));

        // Current generation matches → should disconnect
        pool.disconnect_worker_if_generation(&w1, 0).await.unwrap();
        assert_eq!(pool.worker_state(&w1).await, Some(WorkerState::Disconnected));
    }

    #[tokio::test]
    async fn test_remove_stale_disconnected() {
        let pool = WorkersPool::new();
        let (w1, _) = register_test_worker(&pool, "w1").await;

        pool.disconnect_worker(&w1).await.unwrap();

        // Set heartbeat to 10 minutes ago
        {
            let mut workers = pool.workers.write().await;
            workers.get_mut(&w1).unwrap().last_heartbeat =
                Utc::now() - chrono::Duration::seconds(600);
        }

        pool.remove_stale_disconnected(chrono::Duration::seconds(300)).await;
        assert_eq!(pool.num_workers().await, 0);
    }

    #[tokio::test]
    async fn test_remove_stale_keeps_recent() {
        let pool = WorkersPool::new();
        let (w1, _) = register_test_worker(&pool, "w1").await;

        pool.disconnect_worker(&w1).await.unwrap();
        // Heartbeat is fresh (just disconnected) — should be retained
        pool.remove_stale_disconnected(chrono::Duration::seconds(300)).await;
        assert_eq!(pool.num_workers().await, 1);
    }
}
