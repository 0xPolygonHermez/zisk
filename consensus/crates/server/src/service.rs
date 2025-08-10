use anyhow::Result;
use async_stream::stream;
use consensus_api::{consensus_api_server::*, *};
use consensus_comm::CommManager;
use consensus_config::Config;
use consensus_core::{ComputeCapacity, ProverId, ProverManager, ProverManagerConfig};

use chrono::{DateTime, Utc};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::{pin::Pin, sync::Arc, time::SystemTime};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, instrument};

/// Represents the runtime state of the service
#[derive(Debug)]
pub struct ConsensusService {
    pub _config: Config,
    pub start_time_utc: DateTime<Utc>,
    pub comm_manager: Arc<CommManager>,
    pub active_connections: Arc<AtomicU32>,
    pub prover_manager: Arc<ProverManager>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub version: String,
    pub build_time: String,
    pub commit_hash: String,
    pub environment: String,
    pub api_version: String,
}

impl ConsensusService {
    #[instrument(skip(config))]
    pub async fn new(config: Config) -> consensus_core::Result<Self> {
        info!("Initializing service state");

        let start_time_utc = Utc::now();

        let comm_manager = Arc::new(CommManager::new(config.comm.clone()).await?);

        // Create ProverManager with configuration from config
        let prover_manager_config = ProverManagerConfig::from_config(&config.prover_manager);
        let prover_manager = Arc::new(ProverManager::new(prover_manager_config));

        Ok(Self {
            _config: config,
            start_time_utc,
            comm_manager,
            active_connections: Arc::new(AtomicU32::new(0)),
            prover_manager,
        })
    }

    /// Check if the request comes from localhost/127.0.0.1
    fn is_local_request(&self, request: &Request<impl std::fmt::Debug>) -> bool {
        if let Some(remote_addr) = request.remote_addr() {
            let ip = remote_addr.ip();
            ip.is_loopback() || ip.to_string() == "127.0.0.1" || ip.to_string() == "::1"
        } else {
            false
        }
    }

    /// Validate admin request access
    fn validate_admin_request<T: std::fmt::Debug>(
        &self,
        request: &Request<T>,
    ) -> Result<(), Status> {
        if !self.is_local_request(request) {
            return Err(Status::permission_denied(
                "Admin endpoints are restricted to localhost access only",
            ));
        }
        Ok(())
    }

    /// Handle registration directly in stream context (static version to avoid lifetime issues)
    async fn handle_stream_registration(
        prover_manager: &ProverManager,
        req: ProverRegisterRequest,
        msg_sender: mpsc::Sender<CoordinatorMessage>,
    ) -> Result<ProverId, Status> {
        let compute_capacity = req.compute_capacity.unwrap_or_default();

        prover_manager
            .register_prover(ProverId::from(req.prover_id), compute_capacity, msg_sender)
            .await
            .map_err(|e| Status::internal(format!("Registration failed: {e}")))
    }

    /// Handle reconnection directly in stream context (static version to avoid lifetime issues)
    async fn handle_stream_reconnection(
        prover_manager: &Arc<ProverManager>,
        req: ProverReconnectRequest,
        msg_sender: mpsc::Sender<CoordinatorMessage>,
    ) -> Result<ProverId, Status> {
        let compute_capacity = req.compute_capacity.unwrap_or_default();

        prover_manager
            .register_prover(ProverId::from(req.prover_id), compute_capacity, msg_sender)
            .await
            .map_err(|e| Status::internal(format!("Reconnection failed: {e}")))
    }
}

/// gRPC Service layer - handles transport and delegates to ConsensusService
#[tonic::async_trait]
impl ConsensusApi for ConsensusService {
    type ProverStreamStream =
        Pin<Box<dyn Stream<Item = Result<CoordinatorMessage, Status>> + Send>>;

    async fn status_info(
        &self,
        request: Request<StatusInfoRequest>,
    ) -> Result<Response<StatusInfoResponse>, Status> {
        self.validate_admin_request(&request)?;

        let uptime_seconds = (Utc::now() - self.start_time_utc).num_seconds() as u64;

        let metrics =
            Metrics { active_connections: self.active_connections.load(Ordering::SeqCst) };

        let response = StatusInfoResponse {
            service_name: "Consensus Service".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds,
            start_time: Some(SystemTime::from(self.start_time_utc).into()),
            metrics: Some(metrics),
        };

        Ok(Response::new(response))
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = HealthCheckResponse {};

        Ok(Response::new(response))
    }

    async fn jobs_list(
        &self,
        request: Request<JobsListRequest>,
    ) -> Result<Response<JobsListResponse>, Status> {
        self.validate_admin_request(&request)?;

        // TODO: Implement actual job retrieval from database
        let all_jobs: consensus_core::Result<Vec<consensus_api::JobStatus>> = Ok(vec![]);

        let response = match all_jobs {
            Ok(jobs) => {
                let job_statuses: Vec<consensus_api::JobStatus> = jobs;
                let jobs_list = JobsList { jobs: job_statuses };
                JobsListResponse { result: Some(jobs_list_response::Result::JobsList(jobs_list)) }
            }
            Err(_e) => {
                let error_response = ErrorResponse {
                    code: "INTERNAL_ERROR".to_string(),
                    message: "Failed to retrieve jobs".to_string(),
                };
                JobsListResponse { result: Some(jobs_list_response::Result::Error(error_response)) }
            }
        };

        Ok(Response::new(response))
    }

    async fn provers_list(
        &self,
        request: Request<ProversListRequest>,
    ) -> Result<Response<ProversListResponse>, Status> {
        self.validate_admin_request(&request)?;

        // TODO: Implement actual prover retrieval from database
        let all_provers: consensus_core::Result<Vec<consensus_api::ProverStatus>> = Ok(vec![]);

        let response = match all_provers {
            Ok(provers) => {
                let prover_statuses: Vec<consensus_api::ProverStatus> = provers;
                let provers_list = ProversList { provers: prover_statuses };
                ProversListResponse {
                    result: Some(provers_list_response::Result::ProversList(provers_list)),
                }
            }
            Err(_e) => {
                let error_response = ErrorResponse {
                    code: "INTERNAL_ERROR".to_string(),
                    message: "Failed to retrieve provers".to_string(),
                };
                ProversListResponse {
                    result: Some(provers_list_response::Result::Error(error_response)),
                }
            }
        };

        Ok(Response::new(response))
    }

    async fn job_status(
        &self,
        request: Request<JobStatusRequest>,
    ) -> Result<Response<JobStatusResponse>, Status> {
        self.validate_admin_request(&request)?;

        let req = request.into_inner();

        // TODO: Implement actual job retrieval from database
        let job: consensus_core::Result<Option<consensus_api::JobStatus>> = Ok(None);

        let response = match job {
            Ok(Some(job)) => {
                JobStatusResponse { result: Some(job_status_response::Result::Job(job)) }
            }
            Ok(None) => {
                let error_response = ErrorResponse {
                    code: "JOB_NOT_FOUND".to_string(),
                    message: format!("Job {} not found", req.job_id),
                };
                JobStatusResponse {
                    result: Some(job_status_response::Result::Error(error_response)),
                }
            }
            Err(_e) => {
                let error_response = ErrorResponse {
                    code: "INTERNAL_ERROR".to_string(),
                    message: "Failed to retrieve job status".to_string(),
                };
                JobStatusResponse {
                    result: Some(job_status_response::Result::Error(error_response)),
                }
            }
        };

        Ok(Response::new(response))
    }

    async fn system_status(
        &self,
        request: Request<SystemStatusRequest>,
    ) -> Result<Response<SystemStatusResponse>, Status> {
        self.validate_admin_request(&request)?;

        // Get actual system status from ProverManager
        let total_provers = self.prover_manager.num_provers().await;
        let total_capacity = self.prover_manager.compute_capacity().await;
        let idle_provers = self.prover_manager.num_provers().await;
        let busy_provers = total_provers.saturating_sub(idle_provers);

        let system_status = consensus_api::SystemStatus {
            total_provers: total_provers as u32,
            compute_capacity: total_capacity.compute_units,
            idle_provers: idle_provers as u32,
            busy_provers: busy_provers as u32,
            active_jobs: 0,                // TODO: Implement actual job counting
            pending_jobs: 0,               // TODO: Implement actual job counting
            completed_jobs_last_minute: 0, // TODO: Implement actual metrics
            job_completion_rate: 0.0,      // TODO: Implement actual metrics
            prover_utilization: if total_provers > 0 {
                (busy_provers as f64) / (total_provers as f64)
            } else {
                0.0
            },
        };

        let response = SystemStatusResponse {
            result: Some(system_status_response::Result::Status(system_status)),
        };

        Ok(Response::new(response))
    }

    async fn start_proof(
        &self,
        request: Request<StartProofRequest>,
    ) -> Result<Response<StartProofResponse>, Status> {
        self.validate_admin_request(&request)?;

        let req = request.into_inner();

        // Assign job through ProverManager - messages are sent directly
        let response = match self
            .prover_manager
            .start_proof(
                req.block_id,
                ComputeCapacity { compute_units: req.compute_units },
                req.input_path,
            )
            .await
        {
            Ok(job_id) => {
                let job_id_str: String = job_id.into();
                info!("Successfully started proof job: {}", job_id_str);
                StartProofResponse { result: Some(start_proof_response::Result::JobId(job_id_str)) }
            }
            Err(e) => {
                error!("Failed to start proof job: {}", e);
                let error_response = ErrorResponse {
                    code: "PROOF_START_FAILED".to_string(),
                    message: format!("Failed to start proof: {e}"),
                };
                StartProofResponse {
                    result: Some(start_proof_response::Result::Error(error_response)),
                }
            }
        };

        Ok(Response::new(response))
    }

    /// Bidirectional stream for prover communication
    async fn prover_stream(
        &self,
        request: Request<Streaming<ProverMessage>>,
    ) -> Result<Response<Self::ProverStreamStream>, Status> {
        // Check connection limits first
        let max_connections = self.prover_manager.config().max_concurrent_connections as usize;

        if self.active_connections.load(Ordering::SeqCst) >= max_connections as u32 {
            return Err(Status::resource_exhausted(format!(
                "Maximum concurrent connections reached: {}/{}",
                self.active_connections.load(Ordering::SeqCst),
                max_connections
            )));
        }

        // Clone Arc references to avoid lifetime issues

        let mut in_stream = request.into_inner();

        let active_connections = self.active_connections.clone();
        let prover_manager = self.prover_manager.clone();
        let response_stream = Box::pin(stream! {
            // Increment connection counter
            active_connections.fetch_add(1, Ordering::SeqCst);

            // Create BOUNDED channel for outbound messages to this prover (for backpressure)
            let buffer_size = prover_manager.config().message_buffer_size as usize;
            let (outbound_sender, mut outbound_receiver) = mpsc::channel::<CoordinatorMessage>(buffer_size);

            // Clean registration handling - wait for prover to introduce itself
            let prover_id = match in_stream.next().await {
                Some(Ok(ProverMessage { payload: Some(prover_message::Payload::Register(req)) })) => {
                    match Self::handle_stream_registration(&prover_manager, req, outbound_sender).await {
                        Ok(prover_id) => {
                            // Send success response
                            yield Ok(CoordinatorMessage {
                                payload: Some(coordinator_message::Payload::RegisterResponse(
                                    ProverRegisterResponse {
                                        prover_id: prover_id.as_string(),
                                        accepted: true,
                                        message: "Registration successful".to_string(),
                                        registered_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                                    }
                                ))
                            });
                            prover_id
                        }
                        Err(status) => {
                            active_connections.fetch_sub(1, Ordering::SeqCst);
                            yield Err(status);
                            return;
                        }
                    }
                }
                Some(Ok(ProverMessage { payload: Some(prover_message::Payload::Reconnect(req)) })) => {
                    match Self::handle_stream_reconnection(&prover_manager, req, outbound_sender).await {
                        Ok(prover_id) => {
                            // Send success response
                            yield Ok(CoordinatorMessage {
                                payload: Some(coordinator_message::Payload::RegisterResponse(
                                    ProverRegisterResponse {
                                        prover_id: prover_id.as_string(),
                                        accepted: true,
                                        message: "Reconnection successful".to_string(),
                                        registered_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                                    }
                                ))
                            });
                            prover_id
                        }
                        Err(status) => {
                            // Cleanup and return error
                            active_connections.fetch_sub(1, Ordering::SeqCst);
                            yield Err(status);
                            return;
                        }
                    }
                }
                Some(Ok(_)) => {
                    // First message was not registration or reconnection
                    active_connections.fetch_sub(1, Ordering::SeqCst);
                    yield Err(Status::invalid_argument("First message must be registration or reconnection"));
                    return;
                }
                Some(Err(e)) => {
                    error!("Error receiving first message: {e}");
                    active_connections.fetch_sub(1, Ordering::SeqCst);
                    yield Err(e);
                    return;
                }
                None => {
                    error!("Stream closed without registration");
                    active_connections.fetch_sub(1, Ordering::SeqCst);
                    yield Err(Status::aborted("Connection closed during handshake"));
                    return;
                }
            };

            info!("Prover {} registered successfully, starting message loop", prover_id);

            // Now handle the rest of the stream with tokio::select!
            loop {
                tokio::select! {
                    // Handle incoming messages from prover
                    incoming_result = in_stream.next() => {
                        match incoming_result {
                            Some(Ok(message)) => {
                                // Handle messages directly through the prover manager (no business logic routing needed)
                                if let Err(e) = prover_manager.handle_prover_message(&prover_id, message).await {
                                    error!("Error handling prover message: {}", e);
                                    yield Err(Status::internal(format!("Error handling message: {e}")));
                                    break; // Break to clean up
                                }
                            }
                            Some(Err(e)) => {
                                error!("Error receiving message from prover {prover_id}: {e}");
                                yield Err(e);
                                break; // Break to clean up
                            }
                            None => {
                                info!("Prover {} stream ended", prover_id);
                                break; // Break out of the loop, ending the stream naturally
                            }
                        }
                    }
                    // Handle outgoing messages to prover (with bounded channel)
                    outbound_result = outbound_receiver.recv() => {
                        match outbound_result {
                            Some(message) => {
                                info!("Sending message to prover {}: {:?}", prover_id, message);
                                yield Ok(message);
                            }
                            None => {
                                // Channel closed, likely service shutdown
                                info!("Outbound channel closed for prover {}", prover_id);
                                break; // Break out of the loop, ending the stream naturally
                            }
                        }
                    }
                }
            }

            // Stream cleanup - this runs when the loop breaks
            info!("Cleaning up prover {} connection", prover_id);

            // Decrement connection counter
            active_connections.fetch_sub(1, Ordering::SeqCst);
            info!("Active connections after cleanup: {}", active_connections.load(Ordering::SeqCst));

            // Perform async cleanup
            if let Err(e) = prover_manager.disconnect_prover(&prover_id).await {
                error!("Failed to handle disconnect for prover {}: {}", prover_id, e);
            }
        });

        Ok(Response::new(response_stream))
    }
}
