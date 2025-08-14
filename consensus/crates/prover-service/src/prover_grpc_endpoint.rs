use anyhow::{anyhow, Result};
use consensus_common::{BlockContext, ProverState};
use consensus_common::{BlockId, JobId};
use consensus_grpc_api::*;
use consensus_prover::{prover_service::ComputationResult, ProverService, ProverServiceConfig};
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{error, info};
use zisk_common::MpiContext;

use crate::config::ProverGrpcEndpointConfig;

pub struct ProverGrpcEndpoint {
    config_endpoint: ProverGrpcEndpointConfig,
    prover_service: ProverService,
}

impl ProverGrpcEndpoint {
    pub async fn new(
        config_endpoint: ProverGrpcEndpointConfig,
        config_service: ProverServiceConfig,
        mpi_context: MpiContext,
    ) -> Result<Self> {
        let prover_service = ProverService::new(
            config_endpoint.prover.prover_id.clone(),
            config_endpoint.prover.compute_capacity,
            config_service,
            mpi_context,
        )?;

        Ok(Self { config_endpoint, prover_service })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting prover client {}", self.config_endpoint.prover.prover_id);

        loop {
            match self.prover_service.get_state() {
                ProverState::Disconnected => {
                    if let Err(e) = self.connect_and_run().await {
                        error!("Connection failed: {}", e);
                        tokio::time::sleep(Duration::from_secs(
                            self.config_endpoint.connection.reconnect_interval_seconds,
                        ))
                        .await;
                    }
                }
                ProverState::Error => {
                    error!("Prover in error state, attempting to reconnect");
                    self.prover_service.set_state(ProverState::Disconnected);
                    tokio::time::sleep(Duration::from_secs(
                        self.config_endpoint.connection.reconnect_interval_seconds,
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
        info!("Connecting to coordinator at {}", self.config_endpoint.server.url);

        let channel =
            Channel::from_shared(self.config_endpoint.server.url.clone())?.connect().await?;
        let mut client = consensus_api_client::ConsensusApiClient::new(channel);

        // Create bidirectional stream
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let request_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(message_receiver);
        let request = Request::new(request_stream);

        let response = client.prover_stream(request).await?;
        let mut response_stream = response.into_inner();

        // Send initial registration
        let connect_message = if let Some(job) = self.prover_service.get_current_job() {
            ProverMessage {
                payload: Some(prover_message::Payload::Reconnect(ProverReconnectRequest {
                    prover_id: self.config_endpoint.prover.prover_id.as_string(),
                    compute_capacity: Some(self.config_endpoint.prover.compute_capacity.into()),
                    last_known_job_id: job.lock().await.job_id.as_string(),
                })),
            }
        } else {
            ProverMessage {
                payload: Some(prover_message::Payload::Register(ProverRegisterRequest {
                    prover_id: self.config_endpoint.prover.prover_id.as_string(),
                    compute_capacity: Some(self.config_endpoint.prover.compute_capacity.into()),
                })),
            }
        };

        message_sender.send(connect_message)?;
        self.prover_service.set_state(ProverState::Connecting);

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

        self.prover_service.set_state(ProverState::Disconnected);
        Ok(())
    }

    pub async fn handle_computation_result(
        &mut self,
        result: ComputationResult,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
    ) -> Result<()> {
        match result {
            ComputationResult::Phase1 { job_id, success, result } => {
                self.send_partial_contribution(job_id, success, result, message_sender).await
            }
            ComputationResult::Phase2 { job_id, success, result } => {
                self.send_proof(job_id, success, result, message_sender).await
            }
        }
    }

    async fn send_partial_contribution(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<Vec<u64>>,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
    ) -> Result<()> {
        if let Some(handle) = self.prover_service.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                assert!(success);
                (vec![RowData { values: data }], String::new())
            }
            Err(e) => {
                assert!(!success);
                (vec![], e.to_string())
            }
        };

        let message = ProverMessage {
            payload: Some(prover_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                prover_id: self.config_endpoint.prover.prover_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::PartialContribution as i32,
                success,
                result_data,
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
        result: Result<Vec<Vec<u64>>>,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
    ) -> Result<()> {
        if let Some(handle) = self.prover_service.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                assert!(success);
                (data.into_iter().map(|v| RowData { values: v }).collect(), String::new())
            }
            Err(e) => {
                assert!(!success);
                (vec![], e.to_string())
            }
        };

        let message = ProverMessage {
            payload: Some(prover_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                prover_id: self.config_endpoint.prover.prover_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::Prove as i32,
                success,
                result_data,
                error_message,
            })),
        };

        message_sender.send(message)?;

        // TODO move this logic to the client
        self.prover_service.set_current_job(None);
        self.prover_service.set_state(ProverState::Idle);

        Ok(())
    }

    async fn send_heartbeat_ack(
        &self,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
    ) -> Result<()> {
        let message = ProverMessage {
            payload: Some(prover_message::Payload::HeartbeatAck(HeartbeatAck {
                timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                prover_id: self.config_endpoint.prover.prover_id.as_string(),
            })),
        };

        message_sender.send(message)?;

        Ok(())
    }

    async fn handle_coordinator_message(
        &mut self,
        message: CoordinatorMessage,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
        computation_tx: &mpsc::UnboundedSender<ComputationResult>,
    ) -> Result<()> {
        if let Some(payload) = message.payload {
            match payload {
                coordinator_message::Payload::RegisterResponse(response) => {
                    if response.accepted {
                        info!("Registration accepted: {}", response.message);
                        self.prover_service.set_state(ProverState::Idle);
                    } else {
                        error!("Registration rejected: {}", response.message);
                        self.prover_service.set_state(ProverState::Error);
                    }
                }
                coordinator_message::Payload::ExecuteTask(request) => {
                    match TaskType::try_from(request.task_type) {
                        Ok(TaskType::PartialContribution) => {
                            self.partial_contribution(computation_tx, request).await;
                        }
                        Ok(TaskType::Prove) => {
                            self.prove(computation_tx, request).await?;
                        }
                        Ok(other) => {
                            error!("Received unexpected task type: {:?}", other);
                            return Err(anyhow!("Unexpected task type: {:?}", other));
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
                            self.prover_service.set_state(ProverState::Idle);
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
        info!("Starting Partial Contribution for job {}", request.job_id);

        // Cancel any existing computation
        self.prover_service.cancel_current_computation();

        // Extract the PartialContribution params
        let params = match request.params {
            Some(execute_task_request::Params::PartialContribution(params)) => params,
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
            params.total_provers,
            params.prover_allocation.into_iter().map(|alloc| alloc.into()).collect(),
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

        println!("Received Phase 2 request: {:?}", request);
        let job = self.prover_service.get_current_job().clone().unwrap().clone();
        let job_id = job.lock().await.job_id.clone();
        assert_eq!(job_id.as_string(), request.job_id, "Job ID mismatch in Phase 2");

        info!("Starting Phase 2 for job {}", job_id);

        // Extract the Prove params
        let prove_params = match request.params {
            Some(execute_task_request::Params::Prove(params)) => params,
            _ => {
                return Err(anyhow!("Expected Prove params for Phase 2 task"));
            }
        };

        let mut challenges = Vec::new();
        for challenge in prove_params.challenges {
            challenges.push(challenge.values);
        }

        let tx = computation_tx.clone();
        self.prover_service
            .set_current_computation(self.prover_service.prove(job, challenges, tx).await);

        Ok(())
    }
}
