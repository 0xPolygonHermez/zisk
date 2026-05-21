use crate::{
    coordinator_errors::{CoordinatorError, CoordinatorResult},
    job_events::{CoordinatorJobEvent, CoordinatorJobResult},
    Coordinator,
};
use chrono::Utc;
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

        // A worker can be `SettingUp` either pending this `SetupProgramAck`
        // or pending a `WorkerRecoveryComplete`. The recovery handshake owns
        // the flip back to Ready in the latter case; setup-ack must not
        // pre-empt it.
        let worker_id = ack.worker_id.clone();
        if self.workers_pool.worker_state(&worker_id).await == Some(WorkerState::SettingUp)
            && !self.pending_recovery.read().await.contains_key(&worker_id)
        {
            let _ = self.workers_pool.mark_worker_with_state(&worker_id, WorkerState::Ready).await;
            info!("[Setup] Worker {} finished setup, now Ready", ack.worker_id);
        }

        // Remove this worker from the pending set and accumulate its VK.
        // Only count the VK if the worker was actually pending for this
        // setup — stray acks from unrelated workers must not poison the VK
        // validation.
        let outcome = {
            let mut pending = self.setup_pending.write().await;
            if let Some(state) = pending.get_mut(&job_id) {
                let was_pending = state.pending.remove(&ack.worker_id);
                if was_pending && ack.success {
                    state.vks.push((ack.worker_id.clone(), ack.vk));
                }
                if state.pending.is_empty() {
                    let vks = std::mem::take(&mut state.vks);
                    let hash_id = state.hash_id.clone();
                    let program_name = state.program_name.clone();
                    let with_hints = state.with_hints;
                    let emulator_only = state.emulator_only;
                    pending.remove(&job_id);
                    Some((vks, hash_id, program_name, with_hints, emulator_only))
                } else {
                    None
                }
            } else {
                // Job already completed or unknown — nothing to do.
                return Ok(());
            }
        };

        if let Some((vks, hash_id, program_name, with_hints, emulator_only)) = outcome {
            self.finalize_setup(&job_id, vks, hash_id, program_name, with_hints, emulator_only)
                .await;
            info!("[Setup] All workers acknowledged setup for job_id {}", ack.job_id);
        }

        Ok(())
    }

    /// Fires the terminal event for a completed `setup_pending` entry —
    /// success (with a single agreed VK) or failure (VKs disagreed or none
    /// arrived). Shared by `handle_stream_setup_program_ack` (normal path)
    /// and `prune_setup_pending_for_lost_worker` (worker disappeared mid-
    /// setup).
    async fn finalize_setup(
        &self,
        job_id: &JobId,
        vks: Vec<(WorkerId, Vec<u8>)>,
        hash_id: String,
        program_name: String,
        with_hints: bool,
        emulator_only: bool,
    ) {
        let event = match validate_setup_vks(job_id.as_str(), vks) {
            Ok(vk) => {
                self.active_setups.write().await.insert(
                    SetupKey::new(hash_id, with_hints, emulator_only),
                    crate::coordinator::ActiveSetup { program_name, vk: vk.clone() },
                );
                CoordinatorJobEvent::Completed(CoordinatorJobResult::Setup { vk })
            }
            Err(e) => {
                error!("[Setup] VK mismatch for job_id {}: {}", job_id, e);
                CoordinatorJobEvent::Failed(e)
            }
        };
        self.fire_job_event(job_id, event).await;
    }

    /// Removes `worker_id` from any in-flight `setup_pending` entries; for
    /// each entry whose pending set becomes empty as a result, finalizes
    /// the setup with whatever VKs have arrived so far. Called when a
    /// worker is conclusively lost (gRPC stream dead and either evicted by
    /// `unregister_worker` or transitioned `Disconnected` after the
    /// reconnect grace period). Without this prune, the setup would hang
    /// until process restart: the worker's eventual `SetupProgramAck` for
    /// this `job_id` is permanently lost (a fresh stream gets a new
    /// `job_id` from `read_all_setup_dtos`), and there is no setup-phase
    /// timeout in the monitor sweep.
    async fn prune_setup_pending_for_lost_worker(&self, worker_id: &WorkerId) {
        let completions = {
            let mut pending = self.setup_pending.write().await;
            let mut completions = Vec::new();
            pending.retain(|job_id, state| {
                if !state.pending.remove(worker_id) {
                    return true;
                }
                if state.pending.is_empty() {
                    let vks = std::mem::take(&mut state.vks);
                    completions.push((
                        job_id.clone(),
                        vks,
                        state.hash_id.clone(),
                        state.program_name.clone(),
                        state.with_hints,
                        state.emulator_only,
                    ));
                    false
                } else {
                    true
                }
            });
            completions
        };

        for (job_id, vks, hash_id, program_name, with_hints, emulator_only) in completions {
            info!(
                "[Setup] Worker {} lost; finalizing in-flight setup {} with collected acks",
                worker_id, job_id
            );
            self.finalize_setup(&job_id, vks, hash_id, program_name, with_hints, emulator_only)
                .await;
        }
    }

    pub(crate) async fn handle_stream_recovery_complete(
        &self,
        worker_id: &WorkerId,
    ) -> CoordinatorResult<()> {
        let was_pending = self.pending_recovery.write().await.remove(worker_id).is_some();
        let state = self.workers_pool.worker_state(worker_id).await;
        let parked = state == Some(WorkerState::SettingUp);

        let should_flip = parked && (was_pending || !self.is_setup_in_flight(worker_id).await);

        if should_flip {
            let _ = self.workers_pool.mark_worker_with_state(worker_id, WorkerState::Ready).await;
            if was_pending {
                info!("[Recovery] Worker {} finished recovery, now Ready", worker_id);
            } else {
                info!(
                    "[Recovery] Worker {} RecoveryComplete with no pending-recovery record but parked SettingUp; flipping Ready (cross-stream race)",
                    worker_id
                );
            }
        } else if was_pending {
            warn!(
                "[Recovery] Worker {} consumed pending-recovery entry but state was {:?} (not SettingUp); leaving worker state unchanged (likely re-dispatched)",
                worker_id, state
            );
        } else {
            warn!(
                "[Recovery] Worker {} sent RecoveryComplete with no pending-recovery record (state={:?}); ignoring",
                worker_id, state
            );
        }
        Ok(())
    }

    async fn is_setup_in_flight(&self, worker_id: &WorkerId) -> bool {
        let pending = self.setup_pending.read().await;
        pending.values().any(|s| s.pending.contains(worker_id))
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

        // A fresh `Register` (as opposed to `Reconnect`) means the worker
        // process has lost its in-flight state. Any leftover
        // `pending_recovery` entry from its previous incarnation is now
        // unanswerable — the new worker won't send
        // `WorkerRecoveryComplete` for a recovery it doesn't know about.
        // Leaving the entry would block dispatch (via the `setup_ack`
        // pending-recovery guard) until the stuck-recovery sweep evicts.
        let cleared = self.pending_recovery.write().await.remove(&worker_id).is_some();
        if cleared {
            info!(
                "[Recovery] Cleared stale pending_recovery entry for worker {} on fresh register",
                worker_id
            );
        }

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

        // KeepComputing preserves the Computing(_) state across the channel
        // swap. Pending-recovery workers stay SettingUp so the dispatcher
        // doesn't pick them up before the queued `WorkerRecoveryComplete`
        // arrives on the new stream.
        let initial_state = if matches!(directive, Some(ReconnectionDirectiveDto::KeepComputing)) {
            self.workers_pool.worker_state(&worker_id).await.unwrap_or(default_state)
        } else if self.pending_recovery.read().await.contains_key(&worker_id) {
            WorkerState::SettingUp
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

    /// If the worker was Computing, fail its job via `fail_job` so every
    /// assigned worker (including this one) is parked in `pending_recovery`
    /// and awaits `WorkerRecoveryComplete` before the dispatcher can re-task
    /// them.
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

    /// Removes a worker entirely from the pool and clears any
    /// `pending_recovery` entry. The worker's stream is dropped; a still-alive
    /// worker process sees a broken stream and can re-enter the pool via
    /// fresh registration (the previous entry is gone, including any
    /// `pending_recovery` bookkeeping). Used both for explicit unregister
    /// requests and for evicting workers that the stuck-recovery sweep gave
    /// up on; unlike `disconnect_worker`, no state is preserved for an
    /// in-flight reconnect.
    pub async fn unregister_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        let worker_state = self.workers_pool.worker_state(worker_id).await;
        self.fail_job_if_computing(worker_id, worker_state, "unregistered").await?;
        self.pending_recovery.write().await.remove(worker_id);
        // Drain any in-flight setups; on permanent eviction the worker
        // will never ACK this job_id (a re-registered process gets a
        // fresh setup id from active_setups).
        self.prune_setup_pending_for_lost_worker(worker_id).await;
        self.workers_pool.unregister_worker(worker_id).await
    }

    /// Transient disconnect — preserves `pending_recovery` so a reconnect
    /// re-parks the worker `SettingUp` until recovery completes.
    pub async fn disconnect_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        let worker_state = self.workers_pool.worker_state(worker_id).await;
        self.fail_job_if_computing(worker_id, worker_state, "disconnected").await?;
        let disconnected = self.workers_pool.disconnect_worker(worker_id).await?;
        if disconnected {
            // Even on a "transient" disconnect the worker's gRPC stream is
            // dead; any SetupProgramAck for this job_id is lost (the
            // worker, on reconnect, gets a fresh job_id from
            // active_setups). Without this prune the in-flight setup
            // hangs forever — there is no setup-phase monitor timeout.
            self.prune_setup_pending_for_lost_worker(worker_id).await;
        }
        Ok(())
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

        // Disconnect with generation check (re-validates under write lock).
        // Only prune in-flight setups if the disconnect actually landed —
        // otherwise the worker reconnected between the check above and
        // this call, and we'd be pruning a still-live worker.
        let disconnected = self
            .workers_pool
            .disconnect_worker_if_generation(worker_id, expected_generation)
            .await?;
        if disconnected {
            self.prune_setup_pending_for_lost_worker(worker_id).await;
        }
        Ok(())
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
        self.workers_pool.update_last_heartbeat(&message.worker_id).await?;

        error!("Worker {} error: {}", message.worker_id, message.error_message);

        // Only a worker assigned to the job may fail it.
        let assigned = {
            let jobs_map = self.jobs.read().await;
            match jobs_map.get(&message.job_id) {
                Some(job_arc) => job_arc.read().await.workers.contains(&message.worker_id),
                None => false,
            }
        };
        if !assigned {
            warn!(
                "Worker {} reported error for job {} it is not assigned to; ignoring",
                message.worker_id, message.job_id
            );
            return Err(CoordinatorError::InvalidRequest(format!(
                "Worker {} not assigned to job {}",
                message.worker_id, message.job_id
            )));
        }

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

        // Late arrival for a resolved job (e.g. `spawn_blocking` finished
        // after `JobCancelled`). The worker is — or shortly will be — parked
        // `SettingUp` and tracked in `pending_recovery` by `fail_job` /
        // `cancel_job`; `WorkerRecoveryComplete` owns the flip back to
        // `Ready`. This branch is non-state-mutating: log, optionally
        // register recovery intent for a self-reporting worker, and
        // defensive-guard against stomping a `Computing(other_live_job, _)`
        // state (which should be unreachable under the parking discipline
        // but is cheap insurance).
        let job_entry = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(&message.job_id).cloned()
        };
        if let Some(job_entry) = job_entry {
            let job = job_entry.read().await;
            if job.state().is_resolved() {
                drop(job);
                drop(job_entry);

                let current = self.workers_pool.worker_state(&message.worker_id).await;

                if let Some(WorkerState::Computing((ref other_job_id, _))) = current {
                    if other_job_id != &message.job_id {
                        // Should be unreachable: the worker shouldn't have
                        // been re-tasked while owing a `WorkerRecoveryComplete`.
                        // If it ever happens, refuse to stomp the live state.
                        warn!(
                            "Late ExecuteTaskResponse for resolved job {} from worker {} \
                             dropped: worker is Computing on live job {} — refusing to stomp",
                            message.job_id, message.worker_id, other_job_id
                        );
                        return Ok(());
                    }
                }

                if message.worker_in_recovery {
                    // Self-reporting recovery — make sure the eventual
                    // `WorkerRecoveryComplete` is honored even if the
                    // failure was detected via this response (and not via
                    // `WorkerError`). Idempotent: `or_insert` preserves the
                    // original park timestamp set by `fail_job`.
                    self.pending_recovery
                        .write()
                        .await
                        .entry(message.worker_id.clone())
                        .or_insert_with(Utc::now);
                }

                info!(
                    "Late ExecuteTaskResponse from worker {} for resolved job {} \
                     (worker_in_recovery={}, state={:?}); awaiting WorkerRecoveryComplete",
                    message.worker_id, message.job_id, message.worker_in_recovery, current
                );
                return Ok(());
            }
        }

        // Handle task failure if needed
        if !message.success {
            return self.handle_task_failure(message).await;
        }

        let Some(result_data) = message.result_data.as_ref() else {
            return Err(CoordinatorError::InvalidRequest(format!(
                "Worker {} reported success for job {} without result_data",
                message.worker_id, message.job_id
            )));
        };
        match result_data {
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
        self.workers_pool.update_last_heartbeat(&message.worker_id).await?;

        // Job must exist AND the worker must be assigned to it. Prevents a
        // worker from injecting / failing a job it is not part of.
        let job_arc = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(&message.job_id).cloned()
        };
        let Some(job_arc) = job_arc else {
            warn!(
                "Received ExecuteTaskResponse for unknown job {} from worker {}",
                message.job_id, message.worker_id
            );
            return Err(CoordinatorError::NotFoundOrInaccessible);
        };
        if !job_arc.read().await.workers.contains(&message.worker_id) {
            warn!(
                "Worker {} sent ExecuteTaskResponse for job {} it is not assigned to; refusing",
                message.worker_id, message.job_id
            );
            return Err(CoordinatorError::InvalidRequest(format!(
                "Worker {} not assigned to job {}",
                message.worker_id, message.job_id
            )));
        }
        Ok(())
    }

    /// Handles task execution failures by failing the job and generating appropriate errors.
    ///
    /// # Parameters
    ///
    /// * `message` - Task response containing failure details and context
    async fn handle_task_failure(&self, message: ExecuteTaskResponseDto) -> CoordinatorResult<()> {
        // Surface the worker's own error_message so it propagates verbatim
        // through JobState::Failed to the prove-client.
        let worker_err = message.error_message.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let reason = match worker_err {
            Some(detail) => {
                format!("Task execution failed on worker {}: {}", message.worker_id, detail)
            }
            None => format!("Task execution failed on worker {} (no detail)", message.worker_id),
        };

        self.fail_job(&message.job_id, &reason).await?;

        Err(CoordinatorError::WorkerError(format!(
            "Worker {} failed task for job {}: {}",
            message.worker_id,
            message.job_id,
            worker_err.unwrap_or("(no detail)")
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
