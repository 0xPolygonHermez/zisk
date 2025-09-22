use crate::{worker_service::ComputationResult, ProverServiceConfig, WorkerService};
use anyhow::{anyhow, Result};
use proofman::{AggProofs, ContributionsInfo};
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

pub struct WorkerGrpcEndpoint {
    worker_service_config: WorkerServiceConfig,
    prover_service: WorkerService,
}

impl WorkerGrpcEndpoint {
    pub async fn new(
        worker_service_config: WorkerServiceConfig,
        prover_service_config: ProverServiceConfig,
    ) -> Result<Self> {
        let prover_service = WorkerService::new(
            worker_service_config.worker.worker_id.clone(),
            worker_service_config.worker.compute_capacity,
            prover_service_config,
        )?;

        Ok(Self { worker_service_config, prover_service })
    }

    pub async fn run(&mut self) -> Result<()> {
        let rank = self.prover_service.local_rank();

        loop {
            if rank == 0 {
                match self.prover_service.get_state() {
                    WorkerState::Disconnected => {
                        if let Err(e) = self.connect_and_run().await {
                            error!("Connection failed: {}", e);
                            tokio::time::sleep(Duration::from_secs(
                                self.worker_service_config.connection.reconnect_interval_seconds,
                            ))
                            .await;
                        }
                    }
                    WorkerState::Error => {
                        error!("Worker in error state, attempting to reconnect");
                        self.prover_service.set_state(WorkerState::Disconnected);
                        tokio::time::sleep(Duration::from_secs(
                            self.worker_service_config.connection.reconnect_interval_seconds,
                        ))
                        .await;
                    }
                    _ => {
                        // Should not reach here
                        break;
                    }
                }
            } else {
                // Non-rank 0 workers are executing inside a cluster and only receives MPI requests
                self.prover_service.receive_mpi_request().await?;
            }
        }

        Ok(())
    }

    async fn connect_and_run(&mut self) -> Result<()> {
        info!("Connecting to coordinator at {}", self.worker_service_config.coordinator.url);

        let channel = Channel::from_shared(self.worker_service_config.coordinator.url.clone())?
            .connect()
            .await?;
        let mut client = zisk_distributed_api_client::ZiskDistributedApiClient::new(channel);

        // Create bidirectional stream
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let request_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(message_receiver);
        let request = Request::new(request_stream);

        let response = client.worker_stream(request).await?;
        let mut response_stream = response.into_inner();

        // Send initial registration
        let connect_message = if let Some(job) = self.prover_service.get_current_job() {
            WorkerMessage {
                payload: Some(worker_message::Payload::Reconnect(WorkerReconnectRequest {
                    worker_id: self.worker_service_config.worker.worker_id.as_string(),
                    compute_capacity: Some(
                        self.worker_service_config.worker.compute_capacity.into(),
                    ),
                    last_known_job_id: job.lock().await.job_id.as_string(),
                })),
            }
        } else {
            WorkerMessage {
                payload: Some(worker_message::Payload::Register(WorkerRegisterRequest {
                    worker_id: self.worker_service_config.worker.worker_id.as_string(),
                    compute_capacity: Some(
                        self.worker_service_config.worker.compute_capacity.into(),
                    ),
                })),
            }
        };

        message_sender.send(connect_message)?;
        self.prover_service.set_state(WorkerState::Connecting);

        // Create channels for computation results
        let (computation_tx, mut computation_rx) = mpsc::unbounded_channel::<ComputationResult>();
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

        info!("Connected to coordinator, entering main loop");

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
        self.prover_service.cancel_current_computation();

        self.prover_service.set_state(WorkerState::Disconnected);
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
            ComputationResult::AggProof { job_id, success, result } => {
                self.send_aggregation(job_id, success, result, message_sender).await
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
        if let Some(handle) = self.prover_service.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                assert!(success);
                (data, String::new())
            }
            Err(e) => {
                assert!(!success);
                (vec![], e.to_string())
            }
        };

        let mut ch = Vec::new();
        for cont in result_data {
            ch.push(Challenges {
                worker_index: cont.worker_index,
                airgroup_id: cont.airgroup_id as u32,
                challenge: cont.challenge.to_vec(),
            });
        }

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_service_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::PartialContribution as i32,
                success,
                result_data: Some(ResultData::Challenges(ChallengesList { challenges: ch })),
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
        if let Some(handle) = self.prover_service.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                assert!(success);
                (
                    // TODO! Fix me, at this point we have to send all woker indexes that has contributed to the aggregated proof(s) sent
                    data.into_iter()
                        .map(|v| Proof {
                            airgroup_id: v.airgroup_id,
                            values: v.proof,
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
                worker_id: self.worker_service_config.worker.worker_id.as_string(),
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
    ) -> Result<()> {
        if let Some(handle) = self.prover_service.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                assert!(success);

                if let Some(final_proof) = data {
                    (
                        Some(ResultData::FinalProof(FinalProofList {
                            final_proofs: final_proof
                                .into_iter()
                                .map(|v| FinalProof { values: v })
                                .collect(),
                        })),
                        String::new(),
                    )
                } else {
                    (None, String::new())
                }
            }
            Err(e) => {
                // ! FIXME, return an error?
                assert!(!success);
                (None, e.to_string())
            }
        };

        let reset_current_job = matches!(
            result_data.as_ref(),
            Some(ResultData::FinalProof(FinalProofList { final_proofs })) if !final_proofs.is_empty()
        );

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_service_config.worker.worker_id.as_string(),
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
            self.prover_service.set_current_job(None);
            self.prover_service.set_state(WorkerState::Idle);
        }

        Ok(())
    }

    async fn send_heartbeat_ack(
        &self,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> Result<()> {
        let message = WorkerMessage {
            payload: Some(worker_message::Payload::HeartbeatAck(HeartbeatAck {
                worker_id: self.worker_service_config.worker.worker_id.as_string(),
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
        if let Some(payload) = message.payload {
            match payload {
                coordinator_message::Payload::RegisterResponse(response) => {
                    if response.accepted {
                        info!("Registration accepted: {}", response.message);
                        self.prover_service.set_state(WorkerState::Idle);
                    } else {
                        self.prover_service.set_state(WorkerState::Error);
                        error!("Registration rejected: {}", response.message);
                        std::process::exit(1);
                    }
                }
                coordinator_message::Payload::ExecuteTask(request) => {
                    match TaskType::try_from(request.task_type) {
                        Ok(TaskType::PartialContribution) => {
                            // Convert request to an own type
                            self.partial_contribution(computation_tx, request).await;
                        }
                        Ok(TaskType::Prove) => {
                            self.prove(computation_tx, request).await?;
                        }
                        Ok(TaskType::Aggregate) => {
                            self.aggregate(computation_tx, request).await?;
                        }
                        Err(_) => {
                            error!("Unknown task type: {}", request.task_type);
                            return Err(anyhow!("Unknown task type: {}", request.task_type));
                        }
                    }
                }
                coordinator_message::Payload::JobCancelled(cancelled) => {
                    info!("Job {} cancelled: {}", cancelled.job_id, cancelled.reason);

                    if let Some(ref job) = self.prover_service.get_current_job() {
                        let cancelled_job_id = JobId::from(cancelled.job_id.clone());
                        if job.lock().await.job_id == cancelled_job_id {
                            self.prover_service.cancel_current_computation();
                            self.prover_service.set_state(WorkerState::Idle);
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
                    tokio::time::sleep(Duration::from_secs(shutdown.grace_period_seconds as u64))
                        .await;
                    return Err(anyhow!("Coordinator requested shutdown: {}", shutdown.reason));
                }
            }
        }
        Ok(())
    }

    pub async fn partial_contribution(
        &mut self,
        computation_tx: &mpsc::UnboundedSender<ComputationResult>,
        request: ExecuteTaskRequest,
    ) {
        info!("Starting Partial Contribution for {}", request.job_id);

        // Cancel any existing computation
        self.prover_service.cancel_current_computation();

        // Extract the PartialContribution params
        let params = match request.params {
            Some(execute_task_request::Params::ContributionParams(params)) => params,
            _ => {
                error!("Expected PartialContribution params for Partial Contribution task");
                return;
            }
        };

        let job_id = JobId::from(request.job_id.clone());
        let block = BlockContext {
            block_id: BlockId::from(params.block_id.clone()),
            input_path: PathBuf::from(params.input_path.clone()),
        };

        let job = self.prover_service.new_job(
            job_id.clone(),
            block.clone(),
            params.rank_id,
            params.total_workers,
            params.worker_allocation,
            params.job_compute_units,
        );

        // Start computation in background task
        let tx = computation_tx.clone();
        self.prover_service.set_current_computation(
            self.prover_service.partial_contribution(job.clone(), tx).await,
        );
    }

    pub async fn prove(
        &mut self,
        computation_tx: &mpsc::UnboundedSender<ComputationResult>,
        request: ExecuteTaskRequest,
    ) -> Result<()> {
        assert!(
            self.prover_service.get_current_job().is_some(),
            "Phase 2 received without current job context"
        );

        let job = self.prover_service.get_current_job().clone().unwrap().clone();
        let job_id = job.lock().await.job_id.clone();
        assert_eq!(job_id.as_string(), request.job_id, "Job ID mismatch in Phase 2");

        info!("Starting Phase 2 for {}", job_id);

        // Extract the Prove params
        let prove_params = match request.params {
            Some(execute_task_request::Params::ProveParams(params)) => params,
            _ => {
                return Err(anyhow!("Expected Prove params for Phase 2 task"));
            }
        };

        let mut cont = Vec::new();
        for ch in prove_params.challenges {
            cont.push(ContributionsInfo {
                worker_index: ch.worker_index,
                airgroup_id: ch.airgroup_id as usize,
                challenge: ch.challenge,
            });
        }

        let tx = computation_tx.clone();
        self.prover_service.set_current_computation(self.prover_service.prove(job, cont, tx).await);

        Ok(())
    }

    pub async fn aggregate(
        &mut self,
        computation_tx: &mpsc::UnboundedSender<ComputationResult>,
        request: ExecuteTaskRequest,
    ) -> Result<()> {
        assert!(
            self.prover_service.get_current_job().is_some(),
            "Aggregate received without current job context"
        );

        let job = self.prover_service.get_current_job().clone().unwrap().clone();
        let job_id = job.lock().await.job_id.clone();

        assert_eq!(job_id.as_string(), request.job_id, "Job ID mismatch in Aggregate");

        // Extract the Aggregate params
        let agg_params = match request.params {
            Some(execute_task_request::Params::AggParams(params)) => params,
            _ => {
                return Err(anyhow!("Expected Aggregate params for Aggregate task"));
            }
        };

        let agg_params = AggregationParams {
            agg_proofs: agg_params
                .agg_proofs
                .unwrap()
                .proofs
                .into_iter()
                .map(|p| AggProofData {
                    worker_idx: p.worker_idx,
                    airgroup_id: p.airgroup_id,
                    values: p.values,
                })
                .collect(),
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

        let tx = computation_tx.clone();
        self.prover_service
            .set_current_computation(self.prover_service.aggregate(job, agg_params, tx).await);

        Ok(())
    }
}
