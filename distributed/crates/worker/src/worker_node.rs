use crate::{
    worker::{ComputationResult, LoopEvent, LoopEventSender},
    ProverConfig, Worker,
};
use anyhow::{anyhow, Result};
use proofman::{AggProofs, ContributionsInfo, WitnessInfo};
use std::path::Path;
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{error, info, warn};
use zisk_cluster_api::contribution_params::InputSource;
use zisk_cluster_api::execute_task_response::ResultData;
use zisk_cluster_api::*;
use zisk_cluster_common::{
    AggProofData, AggregationParams, DataCtx, HintsSourceDto, InputSourceDto, ProofKind,
    StreamDataDto, WorkerState,
};
use zisk_cluster_common::{DataId, JobId};
use zisk_common::{ProgramVK, Proof, ZiskExecutorTime, ZiskPaths};
use zisk_prover_backend::{Asm, Emu, GuestProgram, ZiskBackend, ZiskProver};

use crate::config::WorkerServiceConfig;

pub(crate) trait RecoveryActions {
    fn notify_cluster_cancellation(&self);
    fn cluster_barrier(&self);
    fn wait_until_proofman_ready(&self);
}

impl<T: ZiskBackend + 'static> RecoveryActions for ZiskProver<T> {
    fn notify_cluster_cancellation(&self) {
        ZiskProver::notify_cluster_cancellation(self)
    }
    fn cluster_barrier(&self) {
        ZiskProver::cluster_barrier(self)
    }
    fn wait_until_proofman_ready(&self) {
        ZiskProver::wait_until_proofman_ready(self)
    }
}

// notify_cluster_cancellation is P2P; rank 0 reaches the trailing barrier
// quickly while peer ranks only get there once their stuck task unwinds (the
// ASM semaphore times out after a few seconds). Rank 0 waits at the barrier
// so the coordinator can't observe a "Ready" worker until the whole cluster
// has resynchronised — otherwise a follow-up dispatch races stale broadcasts
// queued behind the cancelled task.
//
pub(crate) fn run_recovery<R: RecoveryActions + ?Sized>(prover: &R) -> Result<()> {
    // ASM cleanup runs inside `executor::execute`'s Err arm; the wake-up
    // signal is sent from `worker::cancel_current_computation`. All this
    // task has to do is notify peers, drain any zombie proofman thread, and
    // barrier with the cluster before advertising `Ready`.
    prover.notify_cluster_cancellation();
    prover.wait_until_proofman_ready();
    prover.cluster_barrier();
    Ok(())
}

pub enum WorkerNode<T: ZiskBackend + 'static> {
    WorkerGrpc(WorkerNodeGrpc<T>),
    WorkerMpi(WorkerNodeMpi<T>),
}

impl<T: ZiskBackend + 'static> WorkerNode<T> {
    pub fn world_rank(&self) -> i32 {
        match self {
            WorkerNode::WorkerGrpc(worker) => worker.world_rank(),
            WorkerNode::WorkerMpi(worker) => worker.world_rank(),
        }
    }
}

impl<T: ZiskBackend + 'static> WorkerNode<T> {
    pub async fn new_emu(
        worker_config: WorkerServiceConfig,
        prover_config: ProverConfig,
    ) -> Result<WorkerNode<Emu>> {
        let worker = Worker::<Emu>::new_emu(prover_config)?;

        if worker.local_rank() == 0 {
            Ok(WorkerNode::WorkerGrpc(WorkerNodeGrpc::<Emu>::new(worker_config, worker).await?))
        } else {
            Ok(WorkerNode::WorkerMpi(WorkerNodeMpi::<Emu>::new(worker).await?))
        }
    }

    pub async fn new_asm(
        worker_config: WorkerServiceConfig,
        prover_config: ProverConfig,
    ) -> Result<WorkerNode<Asm>> {
        let worker = Worker::<Asm>::new_asm(prover_config)?;

        if worker.local_rank() == 0 {
            Ok(WorkerNode::WorkerGrpc(WorkerNodeGrpc::<Asm>::new(worker_config, worker).await?))
        } else {
            Ok(WorkerNode::WorkerMpi(WorkerNodeMpi::<Asm>::new(worker).await?))
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        match self {
            WorkerNode::WorkerGrpc(worker) => worker.run().await,
            WorkerNode::WorkerMpi(worker) => worker.run().await,
        }
    }
}

pub struct WorkerNodeMpi<T: ZiskBackend + 'static> {
    worker: Worker<T>,
}

impl<T: ZiskBackend + 'static> WorkerNodeMpi<T> {
    pub async fn new(worker: Worker<T>) -> Result<Self> {
        Ok(Self { worker })
    }

    pub fn world_rank(&self) -> i32 {
        self.worker.world_rank()
    }

    async fn run(&mut self) -> Result<()> {
        assert!(self.worker.local_rank() != 0, "WorkerMpi should not be run by rank 0");

        loop {
            // Non-rank 0 workers are executing inside a cluster and only receives MPI requests
            self.worker.handle_mpi_broadcast_request().await?;
        }
    }
}

pub struct WorkerNodeGrpc<T: ZiskBackend + 'static> {
    worker_config: WorkerServiceConfig,
    worker: Worker<T>,
}

impl<T: ZiskBackend + 'static> WorkerNodeGrpc<T> {
    pub async fn new(worker_config: WorkerServiceConfig, worker: Worker<T>) -> Result<Self> {
        Ok(Self { worker_config, worker })
    }

    pub fn world_rank(&self) -> i32 {
        self.worker.world_rank()
    }

    pub async fn run(&mut self) -> Result<()> {
        assert!(self.worker.local_rank() == 0, "WorkerNodeGrpc should only be run by rank 0");

        // Process-long channel: tasks scheduled before a stream drop must
        // still be deliverable on the next reconnect.
        let (raw_tx, mut loop_rx) = mpsc::unbounded_channel::<LoopEvent>();
        let loop_tx = LoopEventSender::new(raw_tx);

        loop {
            match self.worker.state() {
                WorkerState::Disconnected => {
                    if let Err(e) = self.connect_and_run(&loop_tx, &mut loop_rx).await {
                        error!("Connection failed: {}", e);
                        tokio::time::sleep(Duration::from_secs(
                            self.worker_config.connection.reconnect_interval_seconds,
                        ))
                        .await;
                    }
                }
                WorkerState::Error => {
                    error!("Worker in error state, attempting to reconnect");
                    self.worker.set_state(WorkerState::Disconnected);
                    tokio::time::sleep(Duration::from_secs(
                        self.worker_config.connection.reconnect_interval_seconds,
                    ))
                    .await;
                }
                _ => {
                    // Should not reach here
                    break;
                }
            }
        }

        Ok(())
    }

    async fn connect_and_run(
        &mut self,
        loop_tx: &LoopEventSender,
        loop_rx: &mut mpsc::UnboundedReceiver<LoopEvent>,
    ) -> Result<()> {
        info!("Connecting to coordinator at {}", self.worker_config.coordinator.url);

        let channel =
            Channel::from_shared(self.worker_config.coordinator.url.clone())?.connect().await?;
        let mut client = zisk_distributed_api_client::ZiskDistributedApiClient::new(channel)
            .max_decoding_message_size(MAX_MESSAGE_SIZE)
            .max_encoding_message_size(MAX_MESSAGE_SIZE);

        // Create bidirectional stream
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let request_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(message_receiver);
        let request = Request::new(request_stream);

        let response = client.worker_stream(request).await?;
        let mut response_stream = response.into_inner();

        // Send initial registration
        let connect_message = if let Some(job) = self.worker.current_job() {
            WorkerMessage {
                payload: Some(worker_message::Payload::Reconnect(WorkerReconnectRequest {
                    worker_id: self.worker_config.worker.worker_id.as_string(),
                    compute_capacity: Some(self.worker_config.worker.compute_capacity.into()),
                    last_known_job_id: Some(job.lock().await.job_id.as_string()),
                })),
            }
        } else {
            WorkerMessage {
                payload: Some(worker_message::Payload::Register(WorkerRegisterRequest {
                    worker_id: self.worker_config.worker.worker_id.as_string(),
                    compute_capacity: Some(self.worker_config.worker.compute_capacity.into()),
                })),
            }
        };

        message_sender.send(connect_message)?;
        self.worker.set_state(WorkerState::Connecting);

        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

        // Main non-blocking event loop
        loop {
            // Take the computation handle out so select! can poll it directly:
            // happy path fires the channel arm (handle put back), panic/cancel
            // fires the handle arm. `biased` orders branches loop events >
            // coordinator messages > task health > heartbeat — a wakeup race
            // can still let the JoinHandle resolve before the channel poll
            // sees the result, so the `Ok(())` arm re-drains via `try_recv`.
            let mut computation_handle = self.worker.take_current_computation();
            tokio::select! {
                biased;

                // Highest priority: events from spawn_blocking / recovery driver
                Some(event) = loop_rx.recv() => {
                    match event {
                        LoopEvent::Computation(result) => {
                            // Happy path: task completed and sent its result.
                            // Drop the handle — no need to await it, the task already finished.
                            drop(computation_handle.take());
                            if let Err(e) = self.handle_computation_result(result, &message_sender, loop_tx).await {
                                error!("Error handling computation result: {}", e);
                                self.report_computation_error(&message_sender, &e.to_string()).await;
                                break;
                            }
                        }
                        LoopEvent::RecoveryComplete(rc) => {
                            // Every site that schedules the recovery driver
                            // clears `current_computation` first, so the handle
                            // is None here and `current_job`/`state` were
                            // already cleaned up. Just forward the message.
                            let rc_for_requeue = rc.clone();
                            let msg = WorkerMessage {
                                payload: Some(worker_message::Payload::RecoveryComplete(rc)),
                            };
                            if let Err(e) = message_sender.send(msg) {
                                warn!("Failed to forward WorkerRecoveryComplete: {e}; re-enqueueing");
                                // Put it back on the process-long channel so the
                                // next stream re-delivers; otherwise the coordinator
                                // stays parked in pending_recovery.
                                if let Err(re) = loop_tx.send_recovery_complete(rc_for_requeue) {
                                    error!("Failed to re-enqueue WorkerRecoveryComplete: {re}");
                                }
                                break;
                            }
                        }
                    }
                }
                // Coordinator messages (task dispatch, cancellation, heartbeat, etc.)
                Some(result) = response_stream.next() => {
                    // Put the handle back before processing (coordinator message handler
                    // may need it, e.g. cancel_current_computation).
                    if let Some(h) = computation_handle.take() {
                        self.worker.set_current_computation(h);
                    }
                    match result {
                        Ok(message) => {
                            if let Err(e) = self.handle_coordinator_message(message, &message_sender, loop_tx).await {
                                error!("Error handling coordinator message: {}", e);
                                self.report_computation_error(&message_sender, &e.to_string()).await;
                                // Only truly fatal errors (e.g. registration rejected) set state
                                // to Error inside the handler — those break the loop and trigger
                                // reconnect. Recoverable errors (task dispatch failures, unknown
                                // task types, etc.) just reset the worker to Ready and keep the
                                // stream alive.
                                if matches!(self.worker.state(), WorkerState::Error) {
                                    break;
                                }
                                self.worker.set_current_job(None);
                                self.worker.set_state(WorkerState::Ready);
                            }
                        }
                        Err(e) => {
                            error!("Error receiving message from coordinator: {}", e);
                            break;
                        }
                    }
                }
                // Monitor the computation task handle directly. Fires on panic,
                // cancellation, or silent exit. `biased` checks loop_rx first,
                // but a cross-thread wakeup race can let the JoinHandle resolve before
                // the channel poll observes a just-sent event — so on `Ok(())` we
                // re-drain the channel before declaring a silent exit.
                join_result = async { computation_handle.as_mut().unwrap().await }, if computation_handle.is_some() => {
                    match join_result {
                        Err(join_error) => {
                            error!("Computation task failed unexpectedly: {}", join_error);
                            self.report_computation_error(&message_sender, &join_error.to_string()).await;
                            self.worker.set_current_job(None);
                            self.worker.set_state(WorkerState::Ready);
                        }
                        Ok(()) => {
                            match loop_rx.try_recv() {
                                Ok(LoopEvent::Computation(result)) => {
                                    if let Err(e) = self.handle_computation_result(result, &message_sender, loop_tx).await {
                                        error!("Error handling computation result: {}", e);
                                        self.report_computation_error(&message_sender, &e.to_string()).await;
                                        break;
                                    }
                                }
                                Ok(LoopEvent::RecoveryComplete(rc)) => {
                                    let rc_for_requeue = rc.clone();
                                    let msg = WorkerMessage {
                                        payload: Some(worker_message::Payload::RecoveryComplete(rc)),
                                    };
                                    if let Err(e) = message_sender.send(msg) {
                                        warn!("Failed to forward WorkerRecoveryComplete: {e}; re-enqueueing");
                                        if let Err(re) = loop_tx.send_recovery_complete(rc_for_requeue) {
                                            error!("Failed to re-enqueue WorkerRecoveryComplete: {re}");
                                        }
                                        break;
                                    }
                                    self.worker.set_current_job(None);
                                    self.worker.set_state(WorkerState::Ready);
                                }
                                Err(_) => {
                                    warn!("Computation task exited without sending an event");
                                    self.worker.set_current_job(None);
                                    self.worker.set_state(WorkerState::Ready);
                                }
                            }
                        }
                    }
                }
                _ = heartbeat_interval.tick() => {
                    // Put the handle back before processing.
                    if let Some(h) = computation_handle.take() {
                        self.worker.set_current_computation(h);
                    }
                    if let Err(e) = self.send_heartbeat_ack(&message_sender).await {
                        error!("Error sending heartbeat: {}", e);
                        break;
                    }
                }
                else => {
                    info!("Stream closed, will reconnect");
                    break;
                }
            }
        }

        self.worker.set_state(WorkerState::Disconnected);
        Ok(())
    }

    pub async fn handle_computation_result(
        &mut self,
        result: ComputationResult,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        match result {
            ComputationResult::Execution { job_id, success, result, task_received_time } => {
                self.send_execution(
                    job_id,
                    success,
                    result,
                    message_sender,
                    loop_tx,
                    task_received_time,
                )
                .await
            }
            ComputationResult::Contribution { job_id, success, result, task_received_time } => {
                self.send_partial_contribution(
                    job_id,
                    success,
                    result,
                    message_sender,
                    loop_tx,
                    task_received_time,
                )
                .await
            }
            ComputationResult::Proofs { job_id, success, result } => {
                self.send_proof(job_id, success, result, message_sender, loop_tx).await
            }
            ComputationResult::AggProof {
                job_id,
                success,
                result,
                executed_steps,
                proof_type,
                instances,
            } => {
                self.send_aggregation(
                    job_id,
                    success,
                    result,
                    message_sender,
                    executed_steps,
                    proof_type,
                    instances,
                )
                .await
            }
        }
    }

    /// Sends a [`WorkerError`] message to the coordinator when a computation task
    /// fails unexpectedly (e.g. panic inside `spawn_blocking`). This ensures the
    /// coordinator learns about the failure immediately instead of waiting for a
    /// heartbeat timeout.
    async fn report_computation_error(
        &self,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        error_message: &str,
    ) {
        let job_id = match self.worker.current_job() {
            Some(job) => job.lock().await.job_id.as_string(),
            None => {
                // No current job — nothing useful to report to coordinator
                warn!(
                    "Computation error without active job (not reported to coordinator): {}",
                    error_message
                );
                return;
            }
        };

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::Error(WorkerError {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id,
                error_message: error_message.to_string(),
            })),
        };

        if let Err(e) = message_sender.send(message) {
            error!("Failed to send WorkerError to coordinator: {}", e);
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_partial_contribution(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<(WitnessInfo, ZiskExecutorTime, Vec<ContributionsInfo>, u64)>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
        task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                if !success {
                    return Err(anyhow!(
                        "Inconsistent state: operation reported failure but returned Ok result"
                    ));
                }
                (data, String::new())
            }
            Err(e) => {
                if success {
                    return Err(anyhow!(
                        "Inconsistent state: operation reported success but returned Err result"
                    ));
                }
                ((WitnessInfo::default(), ZiskExecutorTime::default(), vec![], 0), e.to_string())
            }
        };

        let challenges: Vec<Challenges> = result_data
            .2
            .into_iter()
            .map(|cont| Challenges {
                worker_index: cont.worker_index,
                airgroup_id: cont.airgroup_id as u32,
                challenge: cont.challenge.to_vec(),
            })
            .collect();

        let witness_info = WitnessExecInfo {
            witness_time: result_data.0.witness_time,
            publics: result_data.0.publics,
            proof_values: result_data.0.proof_values,
            summary_info: result_data.0.summary_info,
            total_instances: result_data.3,
        };

        let zisk_execution_time = ZiskExecuteTime {
            total_duration: result_data.1.total_duration as f32,
            execution_duration: result_data.1.execution_duration as f32,
            count_and_plan_duration: result_data.1.count_and_plan_duration as f32,
            count_and_plan_mo_duration: result_data.1.count_and_plan_mo_duration as f32,
            asm_execution_duration: result_data
                .1
                .asm_execution_duration
                .map(|asm_info| AsmExecuteInfo { time: asm_info.time, mhz: asm_info.mhz }),
            task_received_time: task_received_time
                .unwrap_or_else(chrono::Utc::now)
                .timestamp_millis() as f64,
        };

        let task_type = TaskType::PartialContribution as i32;
        let result_data_msg = Some(ResultData::Challenges(ChallengesList {
            challenges,
            witness_info: Some(witness_info),
            zisk_execution_time: Some(zisk_execution_time),
        }));

        let worker_in_recovery = !success && self.owns_recovery_for(&job_id).await;

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type,
                success,
                result_data: result_data_msg,
                error_message,
                worker_in_recovery,
            })),
        };

        message_sender.send(message)?;

        if worker_in_recovery {
            self.spawn_post_failure_recovery(loop_tx.clone());
            // Drop `current_job` so a coordinator-originated `JobCancelled`
            // for the same job_id does not race a still-running
            // `spawn_post_failure_recovery` and emit a premature
            // `WorkerRecoveryComplete`.
            self.worker.set_current_job(None);
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_execution(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<(WitnessInfo, ZiskExecutorTime, u64, u64)>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
        task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                if !success {
                    return Err(anyhow!(
                        "Inconsistent state: operation reported failure but returned Ok result"
                    ));
                }
                (data, String::new())
            }
            Err(e) => {
                if success {
                    return Err(anyhow!(
                        "Inconsistent state: operation reported success but returned Err result"
                    ));
                }
                ((WitnessInfo::default(), ZiskExecutorTime::default(), 0, 0), e.to_string())
            }
        };

        let (witness_info, zisk_exec_time, instances, executed_steps) = result_data;

        let witness_info_msg = WitnessExecInfo {
            witness_time: witness_info.witness_time,
            publics: witness_info.publics,
            proof_values: witness_info.proof_values,
            summary_info: witness_info.summary_info,
            total_instances: instances,
        };

        let zisk_execution_time = ZiskExecuteTime {
            total_duration: zisk_exec_time.total_duration as f32,
            execution_duration: zisk_exec_time.execution_duration as f32,
            count_and_plan_duration: zisk_exec_time.count_and_plan_duration as f32,
            count_and_plan_mo_duration: zisk_exec_time.count_and_plan_mo_duration as f32,
            asm_execution_duration: zisk_exec_time
                .asm_execution_duration
                .map(|asm_info| AsmExecuteInfo { time: asm_info.time, mhz: asm_info.mhz }),
            task_received_time: task_received_time
                .unwrap_or_else(chrono::Utc::now)
                .timestamp_millis() as f64,
        };

        let task_type = TaskType::Execution as i32;
        let result_data_msg = Some(ResultData::Execution(Execution {
            instances,
            executed_steps,
            zisk_execution_time: Some(zisk_execution_time),
            witness_info: Some(witness_info_msg),
        }));

        let worker_in_recovery = !success && self.owns_recovery_for(&job_id).await;

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type,
                success,
                result_data: result_data_msg,
                error_message,
                worker_in_recovery,
            })),
        };

        message_sender.send(message)?;

        if worker_in_recovery {
            self.spawn_post_failure_recovery(loop_tx.clone());
            // See `send_partial_contribution`.
            self.worker.set_current_job(None);
        }

        Ok(())
    }

    async fn send_proof(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<Vec<AggProofs>>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                if !success {
                    return Err(anyhow!(
                        "Inconsistent state: Prove reported failure but returned Ok result"
                    ));
                }
                let proofs: Vec<ProofStark> = data
                    .into_iter()
                    .map(|v| ProofStark {
                        airgroup_id: v.airgroup_id,
                        values: v.proof,
                        // NOTE: in this context we take always the first worker index
                        // because at this time at each send_proof call we are processing
                        // proofs for a single worker
                        worker_idx: v.worker_indexes[0] as u32,
                    })
                    .collect();
                let proof_words: usize = proofs.iter().map(|proof| proof.values.len()).sum();
                info!(
                    "Prepared Prove response for {} (success=true, proofs={}, proof_words={}, approx_payload_bytes={})",
                    job_id,
                    proofs.len(),
                    proof_words,
                    proof_words * std::mem::size_of::<u64>()
                );
                (proofs, String::new())
            }
            Err(e) => {
                if success {
                    return Err(anyhow!(
                        "Inconsistent state: Prove reported success but returned Err result"
                    ));
                }
                (vec![], e.to_string())
            }
        };

        let worker_in_recovery = !success && self.owns_recovery_for(&job_id).await;

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::Prove as i32,
                success,
                result_data: Some(ResultData::Proofs(ProofList { proofs: result_data })),
                error_message,
                worker_in_recovery,
            })),
        };

        message_sender.send(message)?;
        info!("Queued Prove response for {} to coordinator stream", job_id);

        if worker_in_recovery {
            self.spawn_post_failure_recovery(loop_tx.clone());
            // See `send_partial_contribution`.
            self.worker.set_current_job(None);
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_aggregation(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<Option<Vec<Vec<u64>>>>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        executed_steps: u64,
        proof_type: ProofKind,
        instances: u64,
    ) -> Result<()> {
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let mut error_message = String::new();
        let mut reset_current_job = false;

        let result_data = match result {
            Ok(data) => {
                if !success {
                    return Err(anyhow!("Aggregation returned Ok result but reported failure"));
                }

                if let Some(final_proof) = data {
                    reset_current_job = !final_proof.is_empty();

                    let proof_data = if !final_proof.is_empty() {
                        let is_plonk = proof_type == ProofKind::Plonk;
                        let flat_proof: Vec<u64> = final_proof.into_iter().flatten().collect();
                        let minimal = proof_type == ProofKind::VadcopFinalMinimal;
                        let verkey = self.worker.get_vadcop_vk(minimal).unwrap_or_else(|e| {
                            error!("Failed to get vadcop verification key: {}", e);
                            vec![]
                        });
                        match Proof::new_from_vadcop_proof(&flat_proof, minimal, verkey) {
                            Ok(zisk_proof) => {
                                let final_proof: Proof = if is_plonk {
                                    match self
                                        .worker
                                        .prover_arc()
                                        .wrap_proof(&zisk_proof, ProofKind::Plonk)
                                        .run()
                                    {
                                        Ok(wrapped) => wrapped.get_proof().clone(),
                                        Err(e) => {
                                            error!(
                                                "Failed to wrap Plonk proof for {}: {}",
                                                job_id, e
                                            );
                                            zisk_proof
                                        }
                                    }
                                } else {
                                    zisk_proof
                                };
                                bincode::serde::encode_to_vec(
                                    &final_proof,
                                    bincode::config::standard(),
                                )
                                .unwrap_or_default()
                            }
                            Err(e) => {
                                error!("Failed to build Proof: {}", e);
                                vec![]
                            }
                        }
                    } else {
                        vec![]
                    };

                    Some(ResultData::FinalProof(FinalProof {
                        proof_data,
                        executed_steps,
                        instances,
                    }))
                } else {
                    Some(ResultData::FinalProof(FinalProof {
                        proof_data: vec![],
                        executed_steps,
                        instances,
                    }))
                }
            }
            Err(e) => {
                if success {
                    return Err(anyhow!("Aggregation returned Err but reported success"));
                }
                error_message = e.to_string();
                Some(ResultData::FinalProof(FinalProof {
                    proof_data: vec![],
                    executed_steps,
                    instances,
                }))
            }
        };

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::Aggregate as i32,
                success,
                result_data,
                error_message,
                worker_in_recovery: false,
            })),
        };

        message_sender.send(message)?;

        // TODO! move this logic to the client
        if reset_current_job {
            info!("Aggregation task completed for {}", job_id);
            self.worker.set_current_job(None);
            self.worker.set_state(WorkerState::Ready);
        }

        Ok(())
    }

    /// `true` when this failure should drive recovery for `job_id`. Returns
    /// `false` if a `JobCancelled` handler has already cleared `current_job`
    /// (recovery is in flight there) or if the worker has already moved on
    /// to another job. Without this guard, rank 0 fires `run_recovery` twice
    /// for the same cancelled job and its second `cluster_barrier` hangs
    /// against rank 1, which only ever calls `cluster_barrier` once.
    async fn owns_recovery_for(&self, job_id: &JobId) -> bool {
        match self.worker.current_job() {
            Some(job) => job.lock().await.job_id == *job_id,
            None => false,
        }
    }

    /// Drive the cluster recovery handshake (notify peer-rank cancellation,
    /// wait for the proofman thread to drain, cluster_barrier), then signal
    /// `WorkerRecoveryComplete`. The ASM soft reset itself runs inside
    /// `executor::execute`'s Err arm — this task is only the post-cancel sync.
    /// On `RECOVERY_TIMEOUT` we log loudly and drop the completion: the worker
    /// stays wedged `SettingUp` (still heartbeating, so the stale-disconnected
    /// sweep won't reap it), so operator action is required.
    fn spawn_post_failure_recovery(&self, loop_tx: LoopEventSender) {
        let prover = self.worker.prover_arc();
        let worker_id = self.worker_config.worker.worker_id.as_string();
        tokio::spawn(async move {
            warn!("[Recovery] {worker_id}: running cluster cancellation handshake");
            let join = tokio::time::timeout(
                Self::RECOVERY_TIMEOUT,
                tokio::task::spawn_blocking(move || run_recovery(&*prover)),
            )
            .await;
            match join {
                Ok(Ok(Ok(()))) => {
                    info!("[Recovery] {worker_id}: cluster handshake done; signalling Ready");
                    let msg = WorkerRecoveryComplete { worker_id: worker_id.clone() };
                    if let Err(e) = loop_tx.send_recovery_complete(msg) {
                        error!("[Recovery] {worker_id}: enqueue RecoveryComplete failed: {e}");
                    }
                }
                Ok(Ok(Err(e))) => error!(
                    "[Recovery] {worker_id}: cluster handshake failed: {e:#}; worker stays in SettingUp"
                ),
                Ok(Err(e)) => {
                    error!("[Recovery] {worker_id}: recovery task panicked: {e}")
                }
                Err(_) => error!(
                    "[Recovery] {worker_id}: cluster handshake timed out after {:?}; worker is wedged in SettingUp and needs operator attention",
                    Self::RECOVERY_TIMEOUT
                ),
            }
        });
    }

    /// Healthy reset is sub-second; this only fires when the prover is stuck.
    const RECOVERY_TIMEOUT: Duration = Duration::from_secs(300);

    async fn send_heartbeat_ack(
        &self,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> Result<()> {
        let message = WorkerMessage {
            payload: Some(worker_message::Payload::HeartbeatAck(HeartbeatAck {
                worker_id: self.worker_config.worker.worker_id.as_string(),
            })),
        };

        message_sender.send(message)?;

        Ok(())
    }

    async fn handle_coordinator_message(
        &mut self,
        message: CoordinatorMessage,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        if message.payload.is_none() {
            return Err(anyhow!("Received empty message from coordinator"));
        }

        match message.payload.unwrap() {
            coordinator_message::Payload::RegisterResponse(response) => {
                if response.accepted {
                    info!("Registration accepted: {}", response.message);

                    // `clear_current_job` detaches any in-flight `spawn_blocking`;
                    // if we then accept a new dispatch, `prepare_for_new_job`'s
                    // `prover.reset()` would race the still-unwinding task.
                    // Schedule the recovery driver whenever we clobbered a live
                    // computation: it joins the prover (`wait_until_proofman_ready`)
                    // then emits `WorkerRecoveryComplete`, parking us `SettingUp`
                    // on the coordinator side until the drain finishes.
                    let mut needs_recovery_drain = false;
                    match response.directive.map(|d| ReconnectionAction::try_from(d.action)) {
                        Some(Ok(ReconnectionAction::CancelStaleJob)) => {
                            info!("Coordinator directed cancellation of stale job");
                            needs_recovery_drain = self.worker.has_current_computation();
                            self.worker.clear_current_job();
                        }
                        Some(Ok(ReconnectionAction::KeepComputing)) => {
                            info!("Coordinator confirmed active job; keep computing");
                        }
                        Some(Ok(ReconnectionAction::Idle)) | None => {
                            if self.worker.current_job().is_some() {
                                warn!("No cancel directive but worker has stale job; clearing");
                                needs_recovery_drain = self.worker.has_current_computation();
                                self.worker.clear_current_job();
                            }
                        }
                        Some(Err(_)) => {
                            warn!(
                                "Unknown reconciliation action; clearing stale state defensively"
                            );
                            needs_recovery_drain = self.worker.has_current_computation();
                            self.worker.clear_current_job();
                        }
                    }
                    if needs_recovery_drain {
                        self.spawn_post_failure_recovery(loop_tx.clone());
                    }

                    // If the coordinator attached setup info, run setup now — before entering
                    // the main event loop so no compute task can be processed without a guest program.
                    if let Some(setup) = response.setup_program {
                        let worker_id = self.worker_config.worker.worker_id.as_string();
                        let job_id = setup.job_id.clone();
                        let hash_id = setup.hash_id.clone();

                        let (success, error_message, vk) = match self.handle_setup_program(setup) {
                            Ok(vk) => (
                                true,
                                String::new(),
                                vk.vk.iter().flat_map(|w| w.to_le_bytes()).collect(),
                            ),
                            Err(e) => {
                                error!(
                                    "[Setup] job_id {} Failed setup during reconnection for hash_id {}: {}",
                                    job_id, hash_id, e
                                );
                                (false, e.to_string(), Vec::new())
                            }
                        };

                        let ack = WorkerMessage {
                            payload: Some(worker_message::Payload::SetupProgramAck(
                                SetupProgramAck {
                                    vk,
                                    job_id,
                                    worker_id,
                                    hash_id,
                                    success,
                                    error_message,
                                },
                            )),
                        };
                        if let Err(e) = message_sender.send(ack) {
                            warn!("Failed to send SetupProgramAck after reconnection: {}", e);
                        }
                    }

                    self.worker.set_state(WorkerState::Ready);
                } else {
                    self.worker.set_state(WorkerState::Error);
                    error!("Registration rejected: {}", response.message);
                    return Err(anyhow!("Registration rejected: {}", response.message));
                }
            }
            coordinator_message::Payload::ExecuteTask(request) => {
                let job_id = request.job_id.clone();
                let task_type_int = request.task_type;
                let dispatch = match TaskType::try_from(task_type_int) {
                    Ok(TaskType::Execution) => self.execute_only(request, loop_tx).await,
                    Ok(TaskType::PartialContribution) => {
                        self.partial_contribution(request, loop_tx).await
                    }
                    Ok(TaskType::Prove) => self.prove(request, loop_tx).await,
                    Ok(TaskType::Aggregate) => self.aggregate(request, loop_tx).await,
                    Ok(TaskType::Wrap) => self.handle_wrap_task(request, message_sender).await,
                    Err(_) => Err(anyhow!("Unknown task type: {task_type_int}")),
                };

                // Dispatch can fail before any computation starts (cache miss,
                // unknown task type), so `current_job` may not be set and
                // `report_computation_error` would silently drop the report.
                // Send an explicit failure using the request's `job_id`.
                if let Err(e) = dispatch {
                    error!("Failed to dispatch task {job_id}: {e:#}");
                    let response = WorkerMessage {
                        payload: Some(worker_message::Payload::ExecuteTaskResponse(
                            ExecuteTaskResponse {
                                worker_id: self.worker_config.worker.worker_id.as_string(),
                                job_id,
                                task_type: task_type_int,
                                success: false,
                                result_data: None,
                                error_message: e.to_string(),
                                worker_in_recovery: false,
                            },
                        )),
                    };
                    if let Err(send_err) = message_sender.send(response) {
                        warn!("Failed to send dispatch-failure response: {send_err}");
                    }
                    self.worker.set_current_job(None);
                    self.worker.set_state(WorkerState::Ready);
                }
            }
            coordinator_message::Payload::StreamData(stream_data) => {
                self.handle_stream_data(stream_data).await?;
            }
            coordinator_message::Payload::InputStreamData(input_data) => {
                self.handle_input_stream_data(input_data).await?;
            }
            coordinator_message::Payload::JobCancelled(cancelled) => {
                info!("Job {} cancelled: {}", cancelled.job_id, cancelled.reason);

                // Two outcomes once we've matched the cancelled job to ours:
                //  - had_computation=true  → spawn ASM soft-reset so the
                //    wedged spawn_blocking task (likely stuck in MPI/ASM sync
                //    with peer ranks) actually unwinds.
                //  - had_computation=false → no task to soft-reset, but the
                //    coordinator parked us SettingUp and is waiting for a
                //    `WorkerRecoveryComplete`.
                let mut spawn_recovery = false;
                let mut emit_recovery_complete_directly = false;
                if let Some(ref job) = self.worker.current_job() {
                    let cancelled_job_id = JobId::from(cancelled.job_id.clone());

                    if job.lock().await.job_id == cancelled_job_id {
                        let had_computation = self.worker.has_current_computation();
                        self.worker.clear_current_job();
                        self.worker.set_state(WorkerState::Ready);

                        if had_computation {
                            spawn_recovery = true;
                        } else {
                            emit_recovery_complete_directly = true;
                        }
                    }
                }

                let ack = WorkerMessage {
                    payload: Some(worker_message::Payload::JobCancelledAck(JobCancelledAck {
                        worker_id: self.worker_config.worker.worker_id.as_string(),
                        job_id: cancelled.job_id,
                    })),
                };
                if let Err(e) = message_sender.send(ack) {
                    warn!("Failed to send JobCancelledAck: {}", e);
                }

                if spawn_recovery {
                    self.spawn_post_failure_recovery(loop_tx.clone());
                } else if emit_recovery_complete_directly {
                    let rc = WorkerRecoveryComplete {
                        worker_id: self.worker_config.worker.worker_id.as_string(),
                    };
                    if let Err(e) = loop_tx.send_recovery_complete(rc) {
                        warn!("Failed to enqueue WorkerRecoveryComplete on cancel: {e}");
                    }
                }
            }
            coordinator_message::Payload::Heartbeat(_) => {
                // Send heartbeat ack
                self.send_heartbeat_ack(message_sender).await?;
            }
            coordinator_message::Payload::SetupProgram(setup) => {
                let worker_id = self.worker_config.worker.worker_id.as_string();
                let job_id = setup.job_id.clone();
                let hash_id = setup.hash_id.clone();

                let (success, error_message, vk) = match self.handle_setup_program(setup) {
                    Ok(vk) => {
                        (true, String::new(), vk.vk.iter().flat_map(|w| w.to_le_bytes()).collect())
                    }
                    Err(e) => {
                        error!(
                            "[Setup] job_id {} Failed setup for hash_id {}: {}",
                            job_id, hash_id, e
                        );
                        (false, e.to_string(), Vec::new())
                    }
                };

                let ack = WorkerMessage {
                    payload: Some(worker_message::Payload::SetupProgramAck(SetupProgramAck {
                        job_id,
                        worker_id,
                        hash_id,
                        success,
                        error_message,
                        vk,
                    })),
                };
                if let Err(e) = message_sender.send(ack) {
                    warn!("Failed to send SetupProgramAck: {}", e);
                }
            }
            coordinator_message::Payload::Shutdown(shutdown) => {
                info!(
                    "Coordinator shutdown: {} (grace period: {}s)",
                    shutdown.reason, shutdown.grace_period_seconds
                );
                tokio::time::sleep(Duration::from_secs(shutdown.grace_period_seconds as u64)).await;
                return Err(anyhow!("Coordinator requested shutdown: {}", shutdown.reason));
            }
        }

        Ok(())
    }

    /// Handles a `SetupProgram` message from the coordinator.
    ///
    /// Writes the ELF to a content-addressed cache path, reloads the `GuestProgram`, and runs
    /// setup (generates ROM binary files on disk).
    fn handle_setup_program(&mut self, setup: SetupProgram) -> Result<ProgramVK> {
        use std::sync::Arc;

        info!("[Setup] job_id {} Received setup for hash_id {}", setup.job_id, setup.hash_id);

        let elf_path = ZiskPaths::global().elf_cache(&setup.hash_id);

        // The cache path is content-addressed (blake3 of ELF bytes), so if the file already
        // exists it is identical to what we received — skip write and re-setup.
        if !elf_path.exists() {
            if let Some(parent) = elf_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&elf_path, &setup.elf_bytes)?;
        }

        let guest_program =
            Arc::new(GuestProgram::from_bytes(setup.program_name, setup.elf_bytes.clone()));

        // Broadcast ELF to secondary MPI ranks and run setup on all ranks.
        let vk = self.worker.run_setup(
            &setup.hash_id,
            &setup.elf_bytes,
            setup.with_hints,
            setup.emulator_only,
            guest_program,
        )?;

        info!("[Setup] job_id {} Completed setup for hash_id {}", setup.job_id, setup.hash_id);
        Ok(vk)
    }

    pub async fn partial_contribution(
        &mut self,
        request: ExecuteTaskRequest,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        let task_received_time = chrono::Utc::now();
        info!("Starting Partial Contribution for {}", request.job_id);

        // Cancel any existing computation
        self.worker.cancel_current_computation();

        // Extract the PartialContribution params
        let Some(execute_task_request::Params::ContributionParams(params)) = request.params else {
            return Err(anyhow!("Expected ContributionParams for Partial Contribution task"));
        };

        let job_id = JobId::from(request.job_id);
        let input_source = Self::resolve_input_source(
            &self.worker_config.worker.inputs_folder,
            params.input_source,
        )
        .await?;

        let hints_source = if let Some(hints_data) = params.hints_data {
            HintsSourceDto::HintsData(hints_data)
        } else if let Some(hints_path) = &params.hints_path {
            if params.hints_stream {
                HintsSourceDto::HintsStream(hints_path.clone())
            } else {
                let hints_uri = Self::validate_subdir(
                    &self.worker_config.worker.inputs_folder,
                    &PathBuf::from(hints_path),
                )
                .await?;

                HintsSourceDto::HintsPath(hints_uri.to_string_lossy().to_string())
            }
        } else {
            HintsSourceDto::HintsNull
        };

        let with_hints = !matches!(hints_source, HintsSourceDto::HintsNull);
        let is_first_partition = params.worker_allocation.contains(&0);
        self.worker.prepare_for_new_job(&params.hash_id, with_hints, is_first_partition)?;

        let data_ctx =
            DataCtx { data_id: DataId::from(params.data_id), input_source, hints_source };

        let job = self.worker.new_job(
            job_id.clone(),
            params.hash_id,
            data_ctx,
            params.rank_id,
            params.total_workers,
            params.worker_allocation,
            params.job_compute_units,
            Some(task_received_time),
        );

        // Start computation in background task
        self.worker.set_current_computation(
            self.worker.handle_partial_contribution(job, loop_tx.clone()).await?,
        );

        Ok(())
    }

    pub async fn execute_only(
        &mut self,
        request: ExecuteTaskRequest,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        let task_received_time = chrono::Utc::now();
        info!("Starting Execution-only for {}", request.job_id);

        // Cancel any existing computation
        self.worker.cancel_current_computation();

        // Extract the ExecutionParams (reuses ContributionParams structure)
        let Some(execute_task_request::Params::ExecutionParams(params)) = request.params else {
            return Err(anyhow!("Expected ExecutionParams for Execution-only task"));
        };

        let job_id = JobId::from(request.job_id);
        let input_source = Self::resolve_input_source(
            &self.worker_config.worker.inputs_folder,
            params.input_source,
        )
        .await?;

        let hints_source = if let Some(hints_data) = params.hints_data {
            HintsSourceDto::HintsData(hints_data)
        } else if let Some(hints_path) = &params.hints_path {
            if params.hints_stream {
                HintsSourceDto::HintsStream(hints_path.clone())
            } else {
                let hints_uri = Self::validate_subdir(
                    &self.worker_config.worker.inputs_folder,
                    &PathBuf::from(hints_path),
                )
                .await?;

                HintsSourceDto::HintsPath(hints_uri.to_string_lossy().to_string())
            }
        } else {
            HintsSourceDto::HintsNull
        };

        let with_hints = !matches!(hints_source, HintsSourceDto::HintsNull);
        let is_first_partition = params.worker_allocation.contains(&0);
        self.worker.prepare_for_new_job(&params.hash_id, with_hints, is_first_partition)?;

        let data_ctx =
            DataCtx { data_id: DataId::from(params.data_id), input_source, hints_source };

        let job = self.worker.new_job(
            job_id.clone(),
            params.hash_id,
            data_ctx,
            params.rank_id,
            params.total_workers,
            params.worker_allocation,
            params.job_compute_units,
            Some(task_received_time),
        );

        // Start execution-only computation in background task
        self.worker.set_current_computation(
            self.worker.handle_execution_only(job, loop_tx.clone()).await?,
        );

        Ok(())
    }

    /// Validates that a subpath is within the base directory and waits for it to exist.
    ///
    /// This function joins the base directory with the provided subpath, waits for the
    /// resulting file/directory to appear (up to 60 seconds), and validates that the
    /// resolved path is within the base directory to prevent path traversal attacks.
    ///
    /// # Security Considerations
    /// - Joins base and subpath before validation
    /// - Canonicalizes paths to resolve symlinks and relative components (e.g., `..`)
    /// - Validates that the resolved path is within the base directory
    /// - Note: There's a small TOCTOU window between file existence check and canonicalization
    ///   where a file could theoretically be replaced with a malicious symlink
    ///
    /// # Arguments
    /// * `base_dir` - The base directory that must contain the subpath
    /// * `subpath` - The relative path within base_dir (can include subdirectories)
    ///
    /// Resolve `InputSource` from a gRPC request into `InputSourceDto`.
    ///
    /// File paths are validated under `inputs_folder` with a 60 s timeout.
    async fn resolve_input_source(
        inputs_folder: &Path,
        source: Option<InputSource>,
    ) -> Result<InputSourceDto> {
        match source {
            Some(InputSource::InputPath(ref path)) => {
                let validated = Self::validate_subdir(inputs_folder, &PathBuf::from(path)).await?;
                Ok(InputSourceDto::InputPath(validated.to_string_lossy().to_string()))
            }
            Some(InputSource::InputData(data)) => Ok(InputSourceDto::InputData(data)),
            None => Ok(InputSourceDto::InputNull),
        }
    }

    ///
    /// # Returns
    /// * `Ok(PathBuf)` - The validated, canonicalized full path
    /// * `Err` - If the path doesn't appear within timeout or is outside base directory
    async fn validate_subdir(base_dir: &Path, subpath: &Path) -> Result<PathBuf> {
        let base_canonical =
            base_dir.canonicalize().map_err(|e| anyhow!("Inputs folder error: {e}"))?;

        // Join base with subpath to get full path
        let full_path = base_dir.join(subpath);

        // Wait for file to appear (timeout: 60 seconds)
        let timeout = Duration::from_secs(60);
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(500); // Poll every 500ms

        while !full_path.exists() {
            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Input path {:?} (subpath: {:?}) did not appear within {:?}",
                    full_path,
                    subpath,
                    timeout
                ));
            }
            tokio::time::sleep(poll_interval).await;
        }

        info!("Found input path {:?} (elapsed: {:?})", full_path, start.elapsed());

        // Canonicalize immediately after existence check to minimize TOCTOU window
        let path_canonical =
            full_path.canonicalize().map_err(|e| anyhow!("Input path error: {e}"))?;

        // Validate that the canonical path is within the base directory
        if path_canonical.starts_with(&base_canonical) {
            Ok(path_canonical)
        } else {
            Err(anyhow!(
                "Input path {:?} (resolved to {:?}) is outside base directory {:?}",
                subpath,
                path_canonical,
                base_canonical
            ))
        }
    }

    pub async fn prove(
        &mut self,
        request: ExecuteTaskRequest,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        if self.worker.current_job().is_none() {
            return Err(anyhow!("Prove received without current job context"));
        }

        let job = self.worker.current_job().clone().unwrap().clone();
        let job_id = job.lock().await.job_id.clone();
        let job_id_str = job_id.as_string();

        if job_id_str != request.job_id {
            return Err(anyhow!(
                "Job ID mismatch in Prove: expected {}, got {}",
                job_id_str,
                request.job_id
            ));
        }

        info!("Starting Prove for {}", job_id);

        // Extract the Prove params
        let Some(execute_task_request::Params::ProveParams(prove_params)) = request.params else {
            return Err(anyhow!("Expected Prove params for Prove task"));
        };

        let cont: Vec<_> = prove_params
            .challenges
            .into_iter()
            .map(|ch| ContributionsInfo {
                worker_index: ch.worker_index,
                airgroup_id: ch.airgroup_id as usize,
                challenge: ch.challenge,
                aggregated: false,
            })
            .collect();

        self.worker
            .set_current_computation(self.worker.handle_prove(job, cont, loop_tx.clone()).await?);

        Ok(())
    }

    pub async fn aggregate(
        &mut self,
        request: ExecuteTaskRequest,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        if self.worker.current_job().is_none() {
            return Err(anyhow!("Aggregate received without current job context"));
        }

        let job = self.worker.current_job().clone().unwrap().clone();
        let job_id = job.lock().await.job_id.clone();

        if job_id.as_string() != request.job_id {
            return Err(anyhow!(
                "Job ID mismatch in Aggregate: expected {}, got {}",
                job_id.as_string(),
                request.job_id
            ));
        }

        // Extract the Aggregate params
        let Some(execute_task_request::Params::AggParams(agg_params)) = request.params else {
            return Err(anyhow!("Expected AggParams params for Aggregate task"));
        };

        let agg_proofs =
            agg_params.agg_proofs.ok_or_else(|| anyhow!("Missing agg_proofs in AggParams"))?.proofs;
        let agg_proofs: Vec<_> = agg_proofs
            .into_iter()
            .map(|p| AggProofData {
                worker_idx: p.worker_idx,
                airgroup_id: p.airgroup_id,
                values: p.values,
            })
            .collect();

        let agg_params = AggregationParams {
            agg_proofs,
            last_proof: agg_params.last_proof,
            final_proof: agg_params.final_proof,
            proof_type: ProofKind::from(agg_params.proof_type),
        };
        self.worker.set_current_computation(self.worker.handle_aggregate(
            job,
            agg_params,
            loop_tx.clone(),
        ));

        Ok(())
    }

    async fn handle_stream_data(&mut self, stream_data: StreamData) -> Result<()> {
        if self.worker.current_job().is_none() {
            return Err(anyhow!("Stream data received without current job context"));
        }

        let job = self.worker.current_job().unwrap();
        let current_job_id = {
            let job_guard = job.lock().await;
            job_guard.job_id.clone()
        };

        let stream_data_dto: StreamDataDto = stream_data.into();

        if current_job_id != stream_data_dto.job_id {
            return Err(anyhow!(
                "Job ID mismatch in StreamData: expected {}, got {}",
                current_job_id.as_string(),
                stream_data_dto.job_id
            ));
        }

        self.worker.route_stream_data(stream_data_dto).await
    }

    async fn handle_input_stream_data(
        &mut self,
        input_data: zisk_cluster_api::InputStreamData,
    ) -> Result<()> {
        let job = self
            .worker
            .current_job()
            .ok_or_else(|| anyhow!("InputStreamData received without current job context"))?;

        let current_job_id = job.lock().await.job_id.clone();
        let incoming_job_id = JobId::from(input_data.job_id.clone());

        if current_job_id != incoming_job_id {
            return Err(anyhow!(
                "Job ID mismatch in InputStreamData: expected {}, got {}",
                current_job_id.as_string(),
                incoming_job_id
            ));
        }

        // gRPC `InputStreamData` only reaches rank 0. Mirror it to peer ranks
        // via MPI so their ASM children get the same bytes — without this
        // they wait on `chunk_done` until the semaphore times out (~10 s) and
        // every streamed-input job fails on every rank ≠ 0.
        // Wire format matches what `process_hints` broadcasts for hint-driven
        // inputs (see `precompiles/hints/src/hints_processor.rs`): tag byte
        // followed by a borsh-encoded `StreamMessage { data: Vec<u64> }`.
        if !input_data.payload.is_empty() {
            if input_data.payload.len() % 8 != 0 {
                return Err(anyhow!(
                    "InputStreamData payload length {} is not a multiple of 8 bytes",
                    input_data.payload.len()
                ));
            }
            let data_u64: Vec<u64> = input_data
                .payload
                .chunks_exact(8)
                .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                .collect();
            let mut serialized = borsh::to_vec(&(
                crate::worker::WorkerMpiTag::ContributionsInputsStream,
                zisk_cluster_common::StreamMessage { data: data_u64 },
            ))
            .map_err(|e| anyhow!("Failed to serialize input stream broadcast: {e}"))?;
            self.worker.prover_arc().mpi_broadcast(&mut serialized)?;
        }

        self.worker.append_raw_input(&input_data.payload)
    }

    async fn handle_wrap_task(
        &mut self,
        request: ExecuteTaskRequest,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> Result<()> {
        let job_id = JobId::from(request.job_id.clone());

        let Some(execute_task_request::Params::WrapParams(wrap_params)) = request.params else {
            return Err(anyhow!("Expected WrapParams for Wrap task"));
        };

        let proof_data = wrap_params.proof_data;
        let proof_dest = wrap_params.proof_dest;

        let prover = self.worker.prover_arc();
        let worker_id_str = self.worker_config.worker.worker_id.as_string();
        let job_id_str = job_id.as_string();

        let (success, result_data, error_message) = tokio::task::spawn_blocking(move || {
            match Worker::<T>::execute_wrap_task(&prover, proof_data, proof_dest) {
                Ok(wrapped_bytes) => (
                    true,
                    Some(ResultData::WrapResult(zisk_cluster_api::WrapResult {
                        proof_data: wrapped_bytes,
                    })),
                    String::new(),
                ),
                Err(e) => {
                    error!("Wrap task failed for {}: {}", job_id_str, e);
                    (false, None, e.to_string())
                }
            }
        })
        .await?;

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: worker_id_str,
                job_id: job_id.as_string(),
                task_type: TaskType::Wrap as i32,
                success,
                result_data,
                error_message,
                worker_in_recovery: false,
            })),
        };

        message_sender.send(message)?;
        Ok(())
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use std::sync::Mutex;

    struct RecordingProver {
        calls: Mutex<Vec<&'static str>>,
    }

    impl RecordingProver {
        fn new() -> Self {
            Self { calls: Mutex::new(Vec::new()) }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl RecoveryActions for RecordingProver {
        fn notify_cluster_cancellation(&self) {
            self.calls.lock().unwrap().push("notify");
        }

        fn cluster_barrier(&self) {
            self.calls.lock().unwrap().push("barrier");
        }

        fn wait_until_proofman_ready(&self) {
            self.calls.lock().unwrap().push("wait_until_proofman_ready");
        }
    }

    /// Notify must precede the proofman drain so peers blocked in MPI
    /// collectives unwind first. The trailing barrier is what keeps rank 0
    /// from racing ahead of peer ranks before advertising Ready.
    #[test]
    fn run_recovery_calls_notify_drain_barrier() {
        let prover = RecordingProver::new();
        run_recovery(&prover).unwrap();
        assert_eq!(prover.calls(), vec!["notify", "wait_until_proofman_ready", "barrier"]);
    }

    /// Process-long loop channel must redeliver after the original
    /// `message_sender` is dropped (transient disconnect).
    #[tokio::test]
    async fn recovery_complete_channel_survives_message_sender_drop() {
        let (raw_tx, mut loop_rx) = mpsc::unbounded_channel::<LoopEvent>();
        let loop_tx = LoopEventSender::new(raw_tx);
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<WorkerMessage>();

        loop_tx.send_recovery_complete(WorkerRecoveryComplete { worker_id: "w0".into() }).unwrap();

        drop(msg_tx);
        drop(msg_rx);

        let (new_msg_tx, mut new_msg_rx) = mpsc::unbounded_channel::<WorkerMessage>();
        let event =
            loop_rx.recv().await.expect("WorkerRecoveryComplete must persist across reconnect");
        let LoopEvent::RecoveryComplete(rc) = event else {
            panic!("expected RecoveryComplete event");
        };
        new_msg_tx
            .send(WorkerMessage { payload: Some(worker_message::Payload::RecoveryComplete(rc)) })
            .unwrap();

        let forwarded = new_msg_rx.recv().await.unwrap();
        assert!(matches!(forwarded.payload, Some(worker_message::Payload::RecoveryComplete(_))));
    }
}
