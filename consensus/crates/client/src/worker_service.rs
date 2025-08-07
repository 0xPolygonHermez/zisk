use anyhow::{anyhow, Result};
use consensus_api::*;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{error, info, warn};

/// Configuration for the prover client
#[derive(Debug, Clone)]
pub struct ProverConfig {
    pub prover_id: String,
    pub server_address: String,
    pub reconnect_interval_seconds: u64,
    pub heartbeat_timeout_seconds: u64,
    pub capabilities: ProverCapabilities,
}

impl Default for ProverConfig {
    fn default() -> Self {
        Self {
            prover_id: uuid::Uuid::new_v4().to_string(),
            server_address: "http://127.0.0.1:8080".to_string(),
            reconnect_interval_seconds: 5,
            heartbeat_timeout_seconds: 10,
            capabilities: ProverCapabilities {
                cpu_cores_num: num_cpus::get() as u32,
                gpu_num: 0, // TODO: Detect GPU count
            },
        }
    }
}

/// Result from computation tasks
#[derive(Debug)]
pub enum ComputationResult {
    Phase1 { job_id: String, rank_id: u32, result: Result<Vec<u64>> },
    Phase2 { job_id: String, rank_id: u32, result: Result<Vec<u64>> },
}

/// Prover state tracking
#[derive(Debug, Clone, PartialEq)]
pub enum ProverState {
    Disconnected,
    Connecting,
    Idle,
    ComputingPhase1,
    ComputingPhase2,
    Error,
}

/// Current job context
#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: String,
    pub block_id: u64,
    pub rank_id: u32,
    pub total_provers: u32,
    pub phase: String,
}

/// Main prover client that connects to the coordinator
pub struct ProverClient {
    config: ProverConfig,
    state: ProverState,
    current_job: Option<JobContext>,
    current_computation: Option<JoinHandle<()>>,
}

impl ProverClient {
    pub fn new(config: ProverConfig) -> Self {
        Self {
            config,
            state: ProverState::Disconnected,
            current_job: None,
            current_computation: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting prover client {}", self.config.prover_id);

        loop {
            match self.state {
                ProverState::Disconnected => {
                    if let Err(e) = self.connect_and_run().await {
                        error!("Connection failed: {}", e);
                        tokio::time::sleep(Duration::from_secs(
                            self.config.reconnect_interval_seconds,
                        ))
                        .await;
                    }
                }
                ProverState::Error => {
                    error!("Prover in error state, attempting to reconnect");
                    self.state = ProverState::Disconnected;
                    tokio::time::sleep(Duration::from_secs(self.config.reconnect_interval_seconds))
                        .await;
                }
                _ => {
                    // Should not reach here with new design
                    break;
                }
            }
        }
        Ok(())
    }

    async fn connect_and_run(&mut self) -> Result<()> {
        info!("Connecting to coordinator at {}", self.config.server_address);

        let channel = Channel::from_shared(self.config.server_address.clone())?.connect().await?;
        let mut client = consensus_api_client::ConsensusApiClient::new(channel);

        // Create bidirectional stream
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let request_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(message_receiver);
        let request = Request::new(request_stream);

        let response = client.prover_stream(request).await?;
        let mut response_stream = response.into_inner();

        // Send initial registration
        let register_message = if let Some(job) = &self.current_job {
            ProverMessage {
                payload: Some(prover_message::Payload::Reconnect(ProverReconnectRequest {
                    prover_id: self.config.prover_id.clone(),
                    capabilities: Some(self.config.capabilities),
                    last_known_job_id: job.job_id.clone(),
                    last_known_phase: job.phase.clone(),
                    last_known_rank_id: job.rank_id,
                })),
            }
        } else {
            ProverMessage {
                payload: Some(prover_message::Payload::Register(ProverRegisterRequest {
                    prover_id: self.config.prover_id.clone(),
                    capabilities: Some(self.config.capabilities),
                })),
            }
        };

        message_sender.send(register_message)?;
        self.state = ProverState::Connecting;

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

                // Handle computation completion
                Some(result) = computation_rx.recv() => {
                    if let Err(e) = self.handle_computation_result(result, &message_sender).await {
                        error!("Error handling computation result: {}", e);
                        break;
                    }
                }

                // Send periodic heartbeat
                _ = heartbeat_interval.tick() => {
                    if matches!(self.state, ProverState::Idle | ProverState::ComputingPhase1 | ProverState::ComputingPhase2) {
                        if let Err(e) = self.send_heartbeat_ack(&message_sender).await {
                            error!("Error sending heartbeat: {}", e);
                            break;
                        }
                    }
                }

                // Handle stream closure
                else => {
                    info!("Stream closed, will reconnect");
                    break;
                }
            }
        }

        // Cancel any running computation
        if let Some(handle) = self.current_computation.take() {
            handle.abort();
        }

        self.state = ProverState::Disconnected;
        Ok(())
    }

    async fn send_phase1_result(
        &mut self,
        job_id: String,
        rank_id: u32,
        result: Result<Vec<u64>>,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
    ) -> Result<()> {
        let message = match result {
            Ok(data) => ProverMessage {
                payload: Some(prover_message::Payload::Phase1Result(ProvePhase1Result {
                    job_id,
                    prover_id: self.config.prover_id.clone(),
                    rank_id,
                    result_data: data,
                    success: true,
                    error_message: String::new(),
                })),
            },
            Err(e) => ProverMessage {
                payload: Some(prover_message::Payload::Phase1Result(ProvePhase1Result {
                    job_id,
                    prover_id: self.config.prover_id.clone(),
                    rank_id,
                    result_data: vec![],
                    success: false,
                    error_message: e.to_string(),
                })),
            },
        };

        message_sender.send(message)?;
        Ok(())
    }

    async fn send_final_proof(
        &mut self,
        job_id: String,
        rank_id: u32,
        result: Result<Vec<u64>>,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
    ) -> Result<()> {
        let message = match result {
            Ok(data) => ProverMessage {
                payload: Some(prover_message::Payload::FinalProof(FinalProof {
                    job_id: job_id.clone(),
                    prover_id: self.config.prover_id.clone(),
                    rank_id,
                    proof_data: data,
                    success: true,
                    error_message: String::new(),
                })),
            },
            Err(e) => ProverMessage {
                payload: Some(prover_message::Payload::FinalProof(FinalProof {
                    job_id: job_id.clone(),
                    prover_id: self.config.prover_id.clone(),
                    rank_id,
                    proof_data: vec![],
                    success: false,
                    error_message: e.to_string(),
                })),
            },
        };

        message_sender.send(message)?;
        Ok(())
    }

    async fn send_heartbeat_ack(
        &self,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
    ) -> Result<()> {
        let message = ProverMessage {
            payload: Some(prover_message::Payload::HeartbeatAck(HeartbeatAck {
                timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                prover_id: self.config.prover_id.clone(),
            })),
        };

        message_sender.send(message)?;
        Ok(())
    }

    // Getters for status monitoring
    pub fn get_state(&self) -> &ProverState {
        &self.state
    }

    pub fn get_current_job(&self) -> &Option<JobContext> {
        &self.current_job
    }

    pub fn get_prover_id(&self) -> &str {
        &self.config.prover_id
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
                        self.state = ProverState::Idle;
                    } else {
                        error!("Registration rejected: {}", response.message);
                        self.state = ProverState::Error;
                    }
                }
                coordinator_message::Payload::ProvePhase1(phase1) => {
                    info!("Starting Phase 1 for job {} (rank {})", phase1.job_id, phase1.rank_id);

                    // Cancel any existing computation
                    if let Some(handle) = self.current_computation.take() {
                        handle.abort();
                    }

                    self.current_job = Some(JobContext {
                        job_id: phase1.job_id.clone(),
                        block_id: phase1.block_id,
                        rank_id: phase1.rank_id,
                        total_provers: phase1.total_provers,
                        phase: "phase1".to_string(),
                    });

                    self.state = ProverState::ComputingPhase1;

                    // Start computation in background task
                    let job_id = phase1.job_id;
                    let block_id = phase1.block_id;
                    let rank_id = phase1.rank_id;
                    let total_provers = phase1.total_provers;
                    let tx = computation_tx.clone();

                    self.current_computation = Some(tokio::spawn(async move {
                        let result =
                            compute_phase1_task(job_id.clone(), block_id, rank_id, total_provers)
                                .await;
                        let _ = tx.send(ComputationResult::Phase1 { job_id, rank_id, result });
                    }));
                }
                coordinator_message::Payload::ProvePhase2(phase2) => {
                    info!("Starting Phase 2 for job {}", phase2.job_id);

                    if let Some(ref mut job) = self.current_job {
                        if job.job_id == phase2.job_id {
                            // Cancel any existing computation
                            if let Some(handle) = self.current_computation.take() {
                                handle.abort();
                            }

                            job.phase = "phase2".to_string();
                            self.state = ProverState::ComputingPhase2;

                            let job_id = phase2.job_id;
                            let rank_id = job.rank_id;
                            let global_challenge = phase2.global_challenge;
                            let tx = computation_tx.clone();

                            self.current_computation = Some(tokio::spawn(async move {
                                let result =
                                    compute_phase2_task(job_id.clone(), rank_id, global_challenge)
                                        .await;
                                let _ =
                                    tx.send(ComputationResult::Phase2 { job_id, rank_id, result });
                            }));
                        } else {
                            warn!(
                                "Received Phase 2 for different job {} (current: {:?})",
                                phase2.job_id, job.job_id
                            );
                        }
                    } else {
                        warn!("Received Phase 2 but no current job context");
                    }
                }
                coordinator_message::Payload::JobCancelled(cancelled) => {
                    info!("Job {} cancelled: {}", cancelled.job_id, cancelled.reason);

                    if let Some(ref job) = self.current_job {
                        if job.job_id == cancelled.job_id {
                            // Cancel computation
                            if let Some(handle) = self.current_computation.take() {
                                handle.abort();
                            }
                            self.current_job = None;
                            self.state = ProverState::Idle;
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

    async fn handle_computation_result(
        &mut self,
        result: ComputationResult,
        message_sender: &mpsc::UnboundedSender<ProverMessage>,
    ) -> Result<()> {
        match result {
            ComputationResult::Phase1 { job_id, rank_id, result } => {
                self.send_phase1_result(job_id, rank_id, result, message_sender).await?;
                self.current_computation = None;
                self.state = ProverState::Idle;
            }
            ComputationResult::Phase2 { job_id, rank_id, result } => {
                self.send_final_proof(job_id, rank_id, result, message_sender).await?;
                self.current_computation = None;
                self.current_job = None;
                self.state = ProverState::Idle;
            }
        }
        Ok(())
    }
}

// Computation task functions (run in separate tokio tasks)
async fn compute_phase1_task(
    job_id: String,
    block_id: u64,
    rank_id: u32,
    total_provers: u32,
) -> Result<Vec<u64>> {
    info!("Computing Phase 1 for job {} (rank {}/{})", job_id, rank_id, total_provers);

    // TODO: Implement actual Phase 1 computation
    // This is a placeholder that simulates work
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Generate some dummy data based on rank_id and block_id
    let result = vec![
        block_id + rank_id as u64,
        (block_id * 2) ^ (rank_id as u64),
        block_id.wrapping_mul(rank_id as u64 + 1),
    ];

    info!("Phase 1 computation completed for job {}", job_id);
    Ok(result)
}

async fn compute_phase2_task(
    job_id: String,
    rank_id: u32,
    global_challenge: Vec<u64>,
) -> Result<Vec<u64>> {
    info!("Computing Phase 2 for job {} with challenge {:?}", job_id, global_challenge);

    // TODO: Implement actual Phase 2 computation
    // This is a placeholder that simulates work
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Generate proof based on challenge and rank
    let mut proof = Vec::new();
    for challenge in global_challenge {
        proof.push(challenge.wrapping_mul(rank_id as u64 + 1));
        proof.push(challenge ^ 0xABCDEF);
    }

    info!("Phase 2 computation completed for job {}", job_id);
    Ok(proof)
}
