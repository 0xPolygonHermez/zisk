use crate::{worker::ComputationResult, ProverConfig, Worker};
use anyhow::{anyhow, Result};
use proofman::{AggProofs, ContributionsInfo};
use std::path::Path;
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{error, info};
use zisk_distributed_common::{AggProofData, AggregationParams, BlockContext, WorkerState};
use zisk_distributed_common::{BlockId, JobId};
use zisk_distributed_grpc_api::execute_task_response::ResultData;
use zisk_distributed_grpc_api::*;

use crate::config::WorkerServiceConfig;

pub enum WorkerNode {
    WorkerGrpc(WorkerNodeGrpc),
    WorkerMpi(WorkerNodeMpi),
}

impl WorkerNode {
    pub async fn new(
        worker_config: WorkerServiceConfig,
        prover_config: ProverConfig,
    ) -> Result<Self> {
        let worker = Worker::new(
            worker_config.worker.worker_id.clone(),
            worker_config.worker.compute_capacity,
            prover_config,
        )?;

        if worker.local_rank() == 0 {
            Ok(WorkerNode::WorkerGrpc(WorkerNodeGrpc::new(worker_config, worker).await?))
        } else {
            Ok(WorkerNode::WorkerMpi(WorkerNodeMpi::new(worker).await?))
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        match self {
            WorkerNode::WorkerGrpc(worker) => worker.run().await,
            WorkerNode::WorkerMpi(worker) => worker.run().await,
        }
    }
}

pub struct WorkerNodeMpi {
    worker: Worker,
}

impl WorkerNodeMpi {
    pub async fn new(worker: Worker) -> Result<Self> {
        Ok(Self { worker })
    }

    async fn run(&mut self) -> Result<()> {
        assert!(self.worker.local_rank() != 0, "WorkerMpi should not be run by rank 0");

        loop {
            // Non-rank 0 workers are executing inside a cluster and only receives MPI requests
            self.worker.handle_mpi_broadcast_request().await?;
        }
    }
}

pub struct WorkerNodeGrpc {
    worker_config: WorkerServiceConfig,
    worker: Worker,
}

impl WorkerNodeGrpc {
    pub async fn new(worker_config: WorkerServiceConfig, worker: Worker) -> Result<Self> {
        Ok(Self { worker_config, worker })
    }

    pub async fn run(&mut self) -> Result<()> {
        assert!(self.worker.local_rank() == 0, "WorkerNodeGrpc should only be run by rank 0");

        loop {
            match self.worker.state() {
                WorkerState::Disconnected => {
                    if let Err(e) = self.connect_and_run().await {
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

    async fn connect_and_run(&mut self) -> Result<()> {
        info!("Connecting to coordinator at {}", self.worker_config.coordinator.url);

        let channel =
            Channel::from_shared(self.worker_config.coordinator.url.clone())?.connect().await?;
        let mut client = zisk_distributed_api_client::ZiskDistributedApiClient::new(channel);

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
                    last_known_job_id: job.lock().await.job_id.as_string(),
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

        // Create channels for computation results
        let (computation_tx, mut computation_rx) = mpsc::unbounded_channel::<ComputationResult>();
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

        // Main non-blocking event loop
        loop {
            tokio::select! {
                // Handle incoming coordinator messages
                Some(result) = response_stream.next() => {
                    match result {
                        Ok(message) => {
                            if let Err(e) = self.handle_coordinator_message(message, &message_sender, &computation_tx).await {
                                error!("Error handling coordinator message: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Error receiving message from coordinator: {}", e);
                            break;
                        }
                    }
                }
                Some(result) = computation_rx.recv() => {
                    if let Err(e) = self.handle_computation_result(result, &message_sender).await {
                        error!("Error handling computation result: {}", e);
                        break;
                    }
                }
                _ = heartbeat_interval.tick() => {
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

        // Cancel any running computation
        self.worker.cancel_current_computation();

        self.worker.set_state(WorkerState::Disconnected);
        Ok(())
    }

    pub async fn handle_computation_result(
        &mut self,
        result: ComputationResult,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> Result<()> {
        match result {
            ComputationResult::Challenge { job_id, success, result } => {
                self.send_partial_contribution(job_id, success, result, message_sender).await
            }
            ComputationResult::Proofs { job_id, success, result } => {
                self.send_proof(job_id, success, result, message_sender).await
            }
            ComputationResult::AggProof { job_id, success, result, executed_steps } => {
                self.send_aggregation(job_id, success, result, message_sender, executed_steps).await
            }
        }
    }

    async fn send_partial_contribution(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<Vec<ContributionsInfo>>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
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
                (vec![], e.to_string())
            }
        };

        let challenges: Vec<Challenges> = result_data
            .into_iter()
            .map(|cont| Challenges {
                worker_index: cont.worker_index,
                airgroup_id: cont.airgroup_id as u32,
                challenge: cont.challenge.to_vec(),
            })
            .collect();

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::PartialContribution as i32,
                success,
                result_data: Some(ResultData::Challenges(ChallengesList { challenges })),
                error_message,
            })),
        };

        message_sender.send(message)?;

        Ok(())
    }

    async fn send_proof(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<Vec<AggProofs>>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> Result<()> {
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                assert!(success);
                (
                    data.into_iter()
                        .map(|v| Proof {
                            airgroup_id: v.airgroup_id,
                            values: v.proof,
                            // NOTE: in this context we take always the first worker index
                            // because at this time at each send_proof call we are processing
                            // proofs for a single worker
                            worker_idx: v.worker_indexes[0] as u32,
                        })
                        .collect(),
                    String::new(),
                )
            }
            Err(e) => {
                assert!(!success);
                (vec![], e.to_string())
            }
        };

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::Prove as i32,
                success,
                result_data: Some(ResultData::Proofs(ProofList { proofs: result_data })),
                error_message,
            })),
        };

        message_sender.send(message)?;

        Ok(())
    }

    async fn send_aggregation(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<Option<Vec<Vec<u64>>>>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        executed_steps: u64,
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
                    Some(ResultData::FinalProof(FinalProof {
                        values: final_proof.into_iter().flatten().collect(),
                        executed_steps,
                    }))
                } else {
                    None
                }
            }
            Err(e) => {
                if success {
                    return Err(anyhow!("Aggregation returned Err but reported success"));
                }
                error_message = e.to_string();
                None
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
            })),
        };

        message_sender.send(message)?;

        // TODO! move this logic to the client
        if reset_current_job {
            info!("Aggregation task completed for {}", job_id);
            self.worker.set_current_job(None);
            self.worker.set_state(WorkerState::Idle);
        }

        Ok(())
    }

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
        computation_tx: &mpsc::UnboundedSender<ComputationResult>,
    ) -> Result<()> {
        if message.payload.is_none() {
            return Err(anyhow!("Received empty message from coordinator"));
        }

        match message.payload.unwrap() {
            coordinator_message::Payload::RegisterResponse(response) => {
                if response.accepted {
                    info!("Registration accepted: {}", response.message);
                    self.worker.set_state(WorkerState::Idle);
                } else {
                    self.worker.set_state(WorkerState::Error);
                    error!("Registration rejected: {}", response.message);
                    std::process::exit(1);
                }
            }
            coordinator_message::Payload::ExecuteTask(request) => {
                match TaskType::try_from(request.task_type) {
                    Ok(TaskType::PartialContribution) => {
                        self.partial_contribution(request, computation_tx).await?;
                    }
                    Ok(TaskType::Prove) => {
                        self.prove(request, computation_tx).await?;
                    }
                    Ok(TaskType::Aggregate) => {
                        self.aggregate(request, computation_tx).await?;
                    }
                    Err(_) => {
                        error!("Unknown task type: {}", request.task_type);
                        return Err(anyhow!("Unknown task type: {}", request.task_type));
                    }
                }
            }
            coordinator_message::Payload::JobCancelled(cancelled) => {
                info!("Job {} cancelled: {}", cancelled.job_id, cancelled.reason);

                if let Some(ref job) = self.worker.current_job() {
                    let cancelled_job_id = JobId::from(cancelled.job_id.clone());

                    if job.lock().await.job_id == cancelled_job_id {
                        self.worker.cancel_current_computation();
                        self.worker.set_state(WorkerState::Idle);
                    }
                }
            }
            coordinator_message::Payload::Heartbeat(_) => {
                // Send heartbeat ack
                self.send_heartbeat_ack(message_sender).await?;
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

    pub async fn partial_contribution(
        &mut self,
        request: ExecuteTaskRequest,
        computation_tx: &mpsc::UnboundedSender<ComputationResult>,
    ) -> Result<()> {
        info!("Starting Partial Contribution for {}", request.job_id);

        // Cancel any existing computation
        self.worker.cancel_current_computation();

        // Extract the PartialContribution params
        let Some(execute_task_request::Params::ContributionParams(params)) = request.params else {
            return Err(anyhow!("Expected ContributionParams for Partial Contribution task"));
        };

        let job_id = JobId::from(request.job_id);
        let input_path =
            self.worker_config.worker.inputs_folder.join(PathBuf::from(params.input_path));

        // Validate that input_path is a subdirectory of inputs_folder
        Self::validate_subdir(&self.worker_config.worker.inputs_folder, &input_path)?;

        let block = BlockContext { block_id: BlockId::from(params.block_id), input_path };

        let job = self.worker.new_job(
            job_id,
            block,
            params.rank_id,
            params.total_workers,
            params.worker_allocation,
            params.job_compute_units,
        );

        // Start computation in background task
        self.worker.set_current_computation(
            self.worker.handle_partial_contribution(job.clone(), computation_tx.clone()).await,
        );

        Ok(())
    }

    fn validate_subdir(base: &Path, candidate: &Path) -> Result<()> {
        let base = base.canonicalize().map_err(|e| anyhow!("Inputs folder error: {e}"))?;

        // Timeout 60 seconds
        let timeout = Duration::from_secs(60);
        let start = std::time::Instant::now();

        while !candidate.exists() {
            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Input path {:?} did not appear within {:?}",
                    candidate,
                    timeout
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        // Una vez que existe, canonicaliza y valida
        let candidate = candidate.canonicalize().map_err(|e| anyhow!("Input path error: {e}"))?;

        if candidate.starts_with(&base) {
            Ok(())
        } else {
            Err(anyhow!(
                "Input path {:?} must be a subdirectory of inputs folder {:?}",
                candidate,
                base
            ))
        }
    }

    pub async fn prove(
        &mut self,
        request: ExecuteTaskRequest,
        computation_tx: &mpsc::UnboundedSender<ComputationResult>,
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
            })
            .collect();

        self.worker.set_current_computation(
            self.worker.handle_prove(job, cont, computation_tx.clone()).await,
        );

        Ok(())
    }

    pub async fn aggregate(
        &mut self,
        request: ExecuteTaskRequest,
        computation_tx: &mpsc::UnboundedSender<ComputationResult>,
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
            return Err(anyhow!("Expected ContributionParams params for Aggregate task"));
        };

        let agg_proofs = agg_params.agg_proofs.unwrap().proofs;
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
            verify_constraints: agg_params.verify_constraints,
            aggregation: agg_params.aggregation,
            final_snark: agg_params.final_snark,
            verify_proofs: agg_params.verify_proofs,
            save_proofs: agg_params.save_proofs,
            test_mode: agg_params.test_mode,
            output_dir_path: PathBuf::from(agg_params.output_dir_path),
            minimal_memory: agg_params.minimal_memory,
        };

        self.worker.set_current_computation(
            self.worker.handle_aggregate(job, agg_params, computation_tx.clone()).await,
        );

        Ok(())
    }
}
