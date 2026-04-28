use crate::{
    coordinator_errors::{CoordinatorError, CoordinatorResult},
    job_events::{CoordinatorJobEvent, CoordinatorJobResult},
    Coordinator,
};
use std::sync::atomic::Ordering;
use tracing::{error, info, warn};
use zisk_cluster_common::{
    CoordinatorMessageDto, ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto,
    HeartbeatAckDto, JobId, ReconnectionDirectiveDto, SetupProgramAckDto, SetupProgramDto,
    WorkerErrorDto, WorkerId, WorkerReconnectRequestDto, WorkerRegisterRequestDto, WorkerState,
};
use zisk_common::SetupKey;

/// Trait for sending messages to workers through various communication channels.
///
/// This trait abstracts the message delivery mechanism, allowing different implementations
/// for various communication protocols (WebSocket, gRPC, etc.). Implementations should
/// be thread-safe (`Send + Sync`).
pub trait MessageSender {
    /// Sends a coordinator message to the connected worker.
    ///
    /// # Parameters
    ///
    /// * `msg` - The message to send, containing task assignments or control commands
    fn send(&self, msg: CoordinatorMessageDto) -> CoordinatorResult<()>;
}

impl Coordinator {
    /// Handles a setup program acknowledgement from a worker.
    ///
    /// Called when a worker reports that it has completed (or failed) a setup operation.
    pub(crate) async fn handle_stream_setup_program_ack(
        &self,
        ack: SetupProgramAckDto,
    ) -> CoordinatorResult<()> {
        let job_id = JobId::from(ack.job_id.clone());

        if ack.success {
            info!(
                "[Setup] Worker {} completed setup for job_id {} hash_id {}",
                ack.worker_id, ack.job_id, ack.hash_id
            );
        } else {
            error!(
                "[Setup] Worker {} failed setup for job_id {} hash_id {}: {}",
                ack.worker_id,
                ack.job_id,
                ack.hash_id,
                ack.error_message.as_deref().unwrap_or("unknown error")
            );
        }

        // If this worker was held in SettingUp (registered while an active setup existed),
        // transition it to Idle now so it becomes eligible for job assignment.
        let worker_id = ack.worker_id.clone();
        if self.workers_pool.worker_state(&worker_id).await == Some(WorkerState::SettingUp) {
            let _ = self.workers_pool.mark_worker_with_state(&worker_id, WorkerState::Ready).await;
            info!("[Setup] Worker {} finished setup, now Idle", ack.worker_id);
        }

        // Remove this worker from the pending set and accumulate its VK.
        let outcome = {
            let mut pending = self.setup_pending.write().await;
            if let Some(state) = pending.get_mut(&job_id) {
                state.pending.remove(&ack.worker_id);
                if ack.success {
                    state.vks.push((ack.worker_id.clone(), ack.vk));
                }
                if state.pending.is_empty() {
                    let vks = std::mem::take(&mut state.vks);
                    let hash_id = state.hash_id.clone();
                    let program_name = state.program_name.clone();
                    let with_hints = state.with_hints;
                    pending.remove(&job_id);
                    Some((vks, hash_id, program_name, with_hints))
                } else {
                    None
                }
            } else {
                // Job already completed or unknown — nothing to do.
                return Ok(());
            }
        };

        if let Some((vks, hash_id, program_name, with_hints)) = outcome {
            let event = match validate_setup_vks(&ack.job_id, vks) {
                Ok(vk) => {
                    self.active_setups
                        .write()
                        .await
                        .insert(SetupKey::new(hash_id, with_hints), program_name);
                    CoordinatorJobEvent::Completed(CoordinatorJobResult::Setup { vk })
                }
                Err(e) => {
                    error!("[Setup] VK mismatch for job_id {}: {}", ack.job_id, e);
                    CoordinatorJobEvent::Failed(e)
                }
            };
            self.fire_job_event(&job_id, event).await;
            info!("[Setup] All workers acknowledged setup for job_id {}", ack.job_id);
        }

        Ok(())
    }

    /// Sends cancellation messages to all workers assigned to a job.
    /// Best-effort: logs warnings on failure but continues.
    pub(crate) async fn cancel_job_workers(
        &self,
        worker_ids: &[WorkerId],
        job_id: &JobId,
        reason: &str,
    ) {
        for worker_id in worker_ids {
            let msg = CoordinatorMessageDto::JobCancelled(zisk_cluster_common::JobCancelledDto {
                job_id: job_id.clone(),
                reason: reason.to_string(),
            });
            if let Err(e) = self.workers_pool.send_message(worker_id, msg).await {
                warn!("Failed to send cancellation to worker {}: {}", worker_id, e);
            }
        }
    }

    /// Marks all Computing workers in the list as Ready.
    pub(super) async fn ensure_workers_ready(&self, worker_ids: &[WorkerId]) {
        self.workers_pool.mark_computing_workers_ready(worker_ids).await;
    }

    /// Handles new worker registration. Returns `(accepted, message)`.
    pub async fn handle_stream_registration(
        &self,
        req: WorkerRegisterRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> (bool, String, Option<SetupProgramDto>) {
        self.registrations.fetch_add(1, Ordering::Relaxed);

        let max_connections = self.config.coordinator.max_total_workers as usize;
        if self.workers_pool.num_workers().await >= max_connections {
            return (
                false,
                format!("Maximum concurrent connections reached: ({})", max_connections),
                None,
            );
        }

        // Read all known setups before registering so the initial state is set atomically —
        // no window where the worker is Idle without having a guest program.
        let mut setups = self.read_all_setup_dtos().await;
        let first_setup = if setups.is_empty() { None } else { Some(setups.remove(0)) };
        let initial_state =
            if first_setup.is_some() { WorkerState::SettingUp } else { WorkerState::Idle };

        let worker_id = req.worker_id.clone();
        match self
            .workers_pool
            .register_worker(req.worker_id, req.compute_capacity, msg_sender, initial_state)
            .await
        {
            Ok(()) => {
                // Send any additional known setups (beyond the first) as messages.
                for setup in setups {
                    let msg = CoordinatorMessageDto::SetupProgram(setup);
                    if let Err(e) = self.workers_pool.send_message(&worker_id, msg).await {
                        warn!("[Setup] Failed to send additional setup to {}: {}", worker_id, e);
                    }
                }
                (true, "Registration successful".to_string(), first_setup)
            }
            Err(e) => (false, format!("Registration failed: {e}"), None),
        }
    }

    /// Handles worker reconnection with state reconciliation.
    ///
    /// When a worker reconnects (process survived a disconnect), it may hold stale
    /// `current_job` state. The coordinator reconciles by checking the claimed job
    /// against its own state and returning a directive:
    ///
    /// | Worker claims job X | Coordinator state          | Directive                |
    /// |---------------------|----------------------------|--------------------------|
    /// | None                | —                          | None (idle)              |
    /// | Some(X)             | Job X unknown              | CancelStaleJob           |
    /// | Some(X)             | Job X terminal             | CancelStaleJob           |
    /// | Some(X)             | Job X active, not assigned | CancelStaleJob           |
    /// | Some(X)             | Job X active, assigned     | ResumeComputing          |
    pub async fn handle_stream_reconnection(
        &self,
        req: WorkerReconnectRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> (bool, String, Option<ReconnectionDirectiveDto>, Option<SetupProgramDto>) {
        self.reconnections.fetch_add(1, Ordering::Relaxed);

        // Check max connections — but allow if the worker already exists (reconnection)
        let max_connections = self.config.coordinator.max_total_workers as usize;
        if self.workers_pool.num_workers().await >= max_connections
            && self.workers_pool.worker_state(&req.worker_id).await.is_none()
        {
            return (
                false,
                format!("Maximum concurrent connections reached: ({})", max_connections),
                None,
                None,
            );
        }

        let worker_id = req.worker_id.clone();
        let last_known_job_id = req.last_known_job_id.clone();

        // Compute the directive first so we know whether to preserve the worker's
        // Computing state. Doing this before register_worker avoids a window where
        // the worker briefly appears Idle and becomes eligible for new job dispatch.
        let directive =
            self.compute_reconnection_directive(&worker_id, last_known_job_id.clone()).await;

        // Read all known setups before registering so the initial state is set atomically —
        // no window where the worker is Idle without having a guest program.
        let mut setups = self.read_all_setup_dtos().await;
        let first_setup = if setups.is_empty() { None } else { Some(setups.remove(0)) };
        let default_state =
            if first_setup.is_some() { WorkerState::SettingUp } else { WorkerState::Idle };

        // For a KeepComputing reconnect, preserve the current Computing(job_id, phase)
        // state through the channel swap. For any other outcome, reset to the default.
        let initial_state = if matches!(directive, Some(ReconnectionDirectiveDto::KeepComputing)) {
            self.workers_pool.worker_state(&worker_id).await.unwrap_or(default_state)
        } else {
            default_state
        };

        if let Err(e) = self
            .workers_pool
            .register_worker(req.worker_id, req.compute_capacity, msg_sender, initial_state)
            .await
        {
            return (false, format!("Reconnection failed: {e}"), None, None);
        }

        if let Some(ref d) = directive {
            match d {
                ReconnectionDirectiveDto::CancelStaleJob => {
                    info!("Reconnection of {worker_id}: directing cancellation of stale job");
                }
                ReconnectionDirectiveDto::KeepComputing => {
                    info!("Reconnection of {worker_id}: job still active, keep computing");
                    // If this worker is the aggregator and has an in-flight task, re-send it.
                    // The previous channel may have dropped while the AggParams was in the TCP
                    // send buffer, so the worker never received it.
                    if let Some(job_id) = last_known_job_id.as_ref() {
                        if let Err(e) =
                            self.replay_inflight_agg_task_if_aggregator(&worker_id, job_id).await
                        {
                            warn!(
                                "Failed to replay in-flight agg task for {worker_id} on reconnect: {e}"
                            );
                        }
                    }
                }
                ReconnectionDirectiveDto::Idle => {}
            }
        }

        // Send any additional known setups (beyond the first) as messages.
        for setup in setups {
            let msg = CoordinatorMessageDto::SetupProgram(setup);
            if let Err(e) = self.workers_pool.send_message(&worker_id, msg).await {
                warn!(
                    "[Setup] Failed to send additional setup on reconnect to {}: {}",
                    worker_id, e
                );
            }
        }

        (true, "Reconnection successful".to_string(), directive, first_setup)
    }

    /// Computes the reconciliation directive for a reconnecting worker based on
    /// its claimed `last_known_job_id` vs the coordinator's current job state.
    async fn compute_reconnection_directive(
        &self,
        worker_id: &WorkerId,
        last_known_job_id: Option<JobId>,
    ) -> Option<ReconnectionDirectiveDto> {
        let claimed_job_id = last_known_job_id?;

        let job_entry = {
            let jobs_map = self.jobs.read().await;
            match jobs_map.get(&claimed_job_id) {
                None => {
                    // Coordinator has no record (restarted or job expired)
                    return Some(ReconnectionDirectiveDto::CancelStaleJob);
                }
                Some(entry) => entry.clone(),
            }
        };

        let job = job_entry.read().await;

        if job.state.is_resolved() {
            return Some(ReconnectionDirectiveDto::CancelStaleJob);
        }

        if !job.workers.contains(worker_id) {
            return Some(ReconnectionDirectiveDto::CancelStaleJob);
        }

        // Job is active and worker is still assigned — process survived the
        // disconnect so the computation may still be running. Let the worker
        // continue; the re-established channel will deliver the result.
        Some(ReconnectionDirectiveDto::KeepComputing)
    }

    /// Removes a worker from the active pool and cleans up associated resources.
    ///
    /// Handles worker disconnection or removal by cleaning up state, reallocating
    /// work if necessary, and ensuring system consistency. This method is typically
    /// called when workers disconnect unexpectedly or during graceful shutdowns.
    ///
    /// # Parameters
    ///
    /// * `worker_id` - Unique identifier of the worker to remove
    ///
    /// # Cleanup Operations
    ///
    /// 1. **State Removal**: Removes worker from active pool and associated data structures
    /// 2. **Job Impact Assessment**: Identifies any active jobs that may be affected
    /// 3. **Resource Reallocation**: May trigger job failure or rebalancing depending on job state
    /// 4. **Connection Cleanup**: Releases communication channels and associated resources
    ///
    /// # Impact on Active Jobs
    ///
    /// When a worker is unregistered:
    /// If the worker was computing, fail the associated job.
    /// Returns Ok(()) if the worker was not computing or if the job was already terminal.
    async fn fail_job_if_computing(
        &self,
        worker_id: &WorkerId,
        worker_state: Option<WorkerState>,
        reason: &str,
    ) -> CoordinatorResult<()> {
        if let Some(WorkerState::Computing((job_id, phase))) = worker_state {
            error!(
                "Worker {} {} while computing for job {} in phase {:?}",
                worker_id, reason, job_id, phase
            );
            self.fail_job(&job_id, format!("Worker {} {}", worker_id, reason)).await?;
        }
        Ok(())
    }

    /// Unregisters a worker. If it was computing, fails the associated job.
    pub async fn unregister_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        let worker_state = self.workers_pool.worker_state(worker_id).await;
        self.fail_job_if_computing(worker_id, worker_state, "unregistered").await?;
        self.workers_pool.unregister_worker(worker_id).await
    }

    pub async fn disconnect_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        let worker_state = self.workers_pool.worker_state(worker_id).await;
        self.fail_job_if_computing(worker_id, worker_state, "disconnected").await?;
        self.workers_pool.disconnect_worker(worker_id).await
    }

    /// Generation-aware disconnect for [`ConnectionDropGuard`].
    ///
    /// Checks if the worker's current connection generation matches the expected
    /// generation. If it does, checks if the worker was computing and fails the
    /// associated job, then disconnects the worker. If the generation doesn't match,
    /// this is a stale guard and the call is a no-op.
    pub(crate) async fn disconnect_worker_if_generation(
        &self,
        worker_id: &WorkerId,
        expected_generation: u64,
    ) -> CoordinatorResult<()> {
        // Read the worker state + generation atomically under one lock
        let (generation_matches, worker_state) = {
            match self.workers_pool.worker_state_and_generation(worker_id).await {
                Some((state, gen)) if gen == expected_generation => (true, Some(state)),
                _ => (false, None),
            }
        };

        if !generation_matches {
            return Ok(());
        }

        // If the worker was computing, fail the associated job
        if let Err(e) =
            self.fail_job_if_computing(worker_id, worker_state, "connection dropped").await
        {
            warn!("Failed to fail job on worker {} disconnect: {}", worker_id, e);
        }

        // Disconnect with generation check (re-validates under write lock)
        self.workers_pool.disconnect_worker_if_generation(worker_id, expected_generation).await
    }

    /// Handles heartbeat acknowledgments from workers to maintain liveness tracking.
    ///
    /// Updates the last known heartbeat timestamp for the worker.
    ///
    /// # Parameters
    ///
    /// * `message` - Heartbeat acknowledgment message containing worker ID
    pub(crate) async fn handle_stream_heartbeat_ack(
        &self,
        message: HeartbeatAckDto,
    ) -> CoordinatorResult<()> {
        self.workers_pool.update_last_heartbeat(&message.worker_id).await
    }

    /// Handles error reports from workers and marks associated jobs as failed.
    ///
    /// # Parameters
    ///
    /// * `message` - Worker error message containing job ID, worker ID, and error details
    pub async fn handle_stream_error(&self, message: WorkerErrorDto) -> CoordinatorResult<()> {
        // Update last heartbeat
        self.workers_pool.update_last_heartbeat(&message.worker_id).await?;

        error!("Worker {} error: {}", message.worker_id, message.error_message);

        self.fail_job(&message.job_id, message.error_message).await.map_err(|e| {
            error!("Failed to mark job {} as failed after worker error: {}", message.job_id, e);
            e
        })?;

        Ok(())
    }

    pub(crate) async fn handle_stream_job_cancelled_ack(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> CoordinatorResult<()> {
        self.workers_pool.update_last_heartbeat(worker_id).await?;
        info!("Worker {} acknowledged cancellation of job {}", worker_id, job_id);
        Ok(())
    }

    /// Handles task execution responses from workers and orchestrates job progression.
    ///
    /// # Parameters
    ///
    /// * `message` - Task execution response containing results or failure details
    pub(crate) async fn handle_stream_execute_task_response(
        &self,
        message: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        // Validate and update heartbeat
        self.validate_and_update_heartbeat(&message).await?;

        // If the job is already terminal (Failed/Completed), this is a late arrival
        // (e.g. spawn_blocking finished after JobCancelled). Mark worker Ready and discard.
        let job_entry = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(&message.job_id).cloned()
        };
        if let Some(job_entry) = job_entry {
            let job = job_entry.read().await;
            if job.state().is_resolved() {
                info!(
                    "Ignoring late ExecuteTaskResponse from worker {} for resolved job {}",
                    message.worker_id, message.job_id
                );
                drop(job);
                drop(job_entry);
                self.workers_pool
                    .mark_worker_with_state(&message.worker_id, WorkerState::Ready)
                    .await?;
                return Ok(());
            }
        }

        // Handle task failure if needed
        if !message.success {
            return self.handle_task_failure(message).await;
        }

        match message.result_data {
            ExecuteTaskResponseResultDataDto::Execution(_) => {
                self.handle_execution_completion(message).await
            }
            ExecuteTaskResponseResultDataDto::Challenges(_) => {
                self.handle_contributions_completion(message).await
            }
            ExecuteTaskResponseResultDataDto::Proofs(_) => {
                self.handle_proofs_completion(message).await
            }
            ExecuteTaskResponseResultDataDto::FinalProof(_) => {
                self.handle_aggregation_completion(message).await
            }
            ExecuteTaskResponseResultDataDto::WrapResult(_) => {
                self.handle_wrap_completion(message).await
            }
        }
    }

    /// Validates incoming task response and updates worker heartbeat.
    ///
    /// # Parameters
    ///
    /// * `message` - The task response message from a worker
    async fn validate_and_update_heartbeat(
        &self,
        message: &ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        // Update last heartbeat
        self.workers_pool.update_last_heartbeat(&message.worker_id).await?;

        // Check if job exists
        if !self.jobs.read().await.contains_key(&message.job_id) {
            warn!(
                "Received ExecuteTaskResponse for unknown job {} from worker {}",
                message.job_id, message.worker_id
            );
            return Err(CoordinatorError::NotFoundOrInaccessible);
        }

        Ok(())
    }

    /// Handles task execution failures by failing the job and generating appropriate errors.
    ///
    /// # Parameters
    ///
    /// * `message` - Task response containing failure details and context
    async fn handle_task_failure(&self, message: ExecuteTaskResponseDto) -> CoordinatorResult<()> {
        self.fail_job(&message.job_id, "Task execution failed").await?;

        Err(CoordinatorError::WorkerError(format!(
            "Worker {} failed to execute task for {}: {}",
            message.worker_id,
            message.job_id,
            message.error_message.unwrap_or_default()
        )))
    }
}

/// Validates that all workers produced the same VK and returns it.
/// Returns an error string if there are no VKs (all workers failed) or if VKs disagree.
fn validate_setup_vks(job_id: &str, vks: Vec<(WorkerId, Vec<u8>)>) -> Result<Vec<u8>, String> {
    let mut iter = vks.into_iter();
    let (_, first_vk) = iter
        .next()
        .ok_or_else(|| format!("job {job_id}: all workers failed setup, no VK received"))?;
    for (worker_id, vk) in iter {
        if vk != first_vk {
            return Err(format!(
                "job {job_id}: worker {worker_id} returned a different VK than the first worker"
            ));
        }
    }
    Ok(first_vk)
}
