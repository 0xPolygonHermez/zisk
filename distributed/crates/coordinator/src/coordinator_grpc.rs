//! gRPC transport layer for the distributed proof coordination system.
//!
//! This module provides the gRPC server implementation that exposes the coordinator's
//! functionality over the network, handling bidirectional streaming communication
//! with workers and admin API endpoints.

use async_stream::stream;
use futures_util::{Stream, StreamExt};
use std::{pin::Pin, sync::Arc};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info};
use zisk_distributed_common::{CoordinatorMessageDto, JobId, WorkerId};
use zisk_distributed_grpc_api::{zisk_distributed_api_server::*, *};

use crate::config::Config;
use crate::coordinator::MessageSender;
use crate::coordinator_errors::{CoordinatorError, CoordinatorResult};
use crate::Coordinator;

/// gRPC message sender adapter for worker communication.
///
/// Wraps an unbounded channel sender to implement the MessageSender trait,
/// decoupling gRPC streaming from the core coordinator service.
pub struct GrpcMessageSender(mpsc::UnboundedSender<CoordinatorMessage>);

impl GrpcMessageSender {
    /// Creates a new gRPC message sender with the given channel.
    pub fn new(sender: mpsc::UnboundedSender<CoordinatorMessage>) -> Self {
        Self(sender)
    }
}

impl MessageSender for GrpcMessageSender {
    /// Sends a message to the worker through the gRPC channel.
    ///
    /// # Returns
    ///
    /// `Internal` error if the channel is closed or full.
    fn send(&self, msg: CoordinatorMessageDto) -> CoordinatorResult<()> {
        self.0
            .send(msg.into())
            .map_err(|e| CoordinatorError::Internal(format!("Failed to send message: {}", e)))?;
        Ok(())
    }
}

/// gRPC server implementation for the distributed proof coordination system.
///
/// Serves as the network transport layer, exposing coordinator functionality through:
/// - Admin API endpoints (status, job management, system monitoring)  
/// - Bidirectional streaming for worker communication
/// - Authentication and authorization for admin endpoints
pub struct CoordinatorGrpc {
    coordinator_service: Arc<Coordinator>,
}

impl CoordinatorGrpc {
    /// Creates a new gRPC service instance with the given configuration.
    ///
    /// # Parameters
    ///
    /// * `config` - Configuration parameters for the coordinator service.
    pub async fn new(config: Config) -> CoordinatorResult<Self> {
        Ok(Self { coordinator_service: Arc::new(Coordinator::new(config)) })
    }

    /// Checks if the request originates from localhost for admin endpoint security.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming gRPC request to check.
    ///
    /// # Returns
    ///
    /// `true` if the request is from localhost, `false` otherwise.
    fn is_local_request(&self, request: &Request<impl std::fmt::Debug>) -> bool {
        if let Some(remote_addr) = request.remote_addr() {
            let ip = remote_addr.ip();
            ip.is_loopback() || ip.to_string() == "127.0.0.1" || ip.to_string() == "::1"
        } else {
            false
        }
    }

    /// Validates that admin requests come from localhost only.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming gRPC request to validate.
    ///
    /// # Returns
    ///
    /// `Status::permission_denied` if request is not from localhost.
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

    /// Validates that the worker ID in the message matches the authenticated worker.
    ///
    /// # Parameters
    ///
    /// * `worker_id` - The authenticated worker ID.
    /// * `request_worker_id` - The worker ID from the incoming message.
    ///
    /// # Returns
    ///
    /// `InvalidRequest` error if worker IDs don't match.
    fn validate_same_worker_id(
        worker_id: &WorkerId,
        request_worker_id: &str,
    ) -> CoordinatorResult<()> {
        if worker_id.as_string() != request_worker_id {
            // Log the mismatch internally for debugging
            error!(
                "Worker ID mismatch: expected {}, got {}",
                worker_id.as_string(),
                request_worker_id
            );
            // Return generic error to client (security best practice)
            return Err(CoordinatorError::InvalidRequest("Invalid worker credentials".to_string()));
        }
        Ok(())
    }

    /// Processes individual messages from the worker stream.
    ///
    /// Routes messages to appropriate coordinator handlers after validation.
    ///
    /// # Parameters
    ///
    /// * `coordinator` - Reference to the coordinator service instance.
    /// * `worker_id` - The authenticated worker ID.
    /// * `message` - The incoming worker message to process.
    async fn handle_stream_message(
        coordinator: &Coordinator,
        worker_id: &WorkerId,
        message: WorkerMessage,
    ) -> CoordinatorResult<()> {
        match message.payload {
            Some(payload) => match payload {
                worker_message::Payload::HeartbeatAck(heartbeat_ack) => {
                    Self::validate_same_worker_id(worker_id, &heartbeat_ack.worker_id)?;
                    coordinator.handle_stream_heartbeat_ack(heartbeat_ack.into()).await
                }
                worker_message::Payload::Error(worker_error) => {
                    Self::validate_same_worker_id(worker_id, &worker_error.worker_id)?;
                    coordinator.handle_stream_error(worker_error.into()).await
                }
                worker_message::Payload::Register(_) | worker_message::Payload::Reconnect(_) => {
                    unreachable!("Register/Reconnect should be handled in the initial handshake");
                }
                worker_message::Payload::ExecuteTaskResponse(execute_task_response) => {
                    Self::validate_same_worker_id(worker_id, &execute_task_response.worker_id)?;
                    coordinator
                        .handle_stream_execute_task_response(execute_task_response.into())
                        .await
                }
            },
            None => Err(CoordinatorError::InvalidRequest("Invalid message format".to_string())),
        }
    }

    /// Creates a registration response message for worker handshake.
    ///
    /// # Parameters
    ///
    /// * `worker_id` - The worker ID to include in the response.
    /// * `accepted` - Whether the registration was accepted.
    /// * `message` - Additional message to include in the response.
    fn registration_response(
        worker_id: &WorkerId,
        accepted: bool,
        message: String,
    ) -> Result<CoordinatorMessage, Status> {
        Ok(CoordinatorMessage {
            payload: Some(coordinator_message::Payload::RegisterResponse(WorkerRegisterResponse {
                worker_id: worker_id.as_string(),
                accepted,
                message,
                registered_at: if accepted {
                    Some(prost_types::Timestamp::from(std::time::SystemTime::now()))
                } else {
                    None
                },
            })),
        })
    }
}

/// Implements the gRPC service trait generated by tonic from the protobuf definitions.
#[tonic::async_trait]
impl ZiskDistributedApi for CoordinatorGrpc {
    type WorkerStreamStream =
        Pin<Box<dyn Stream<Item = Result<CoordinatorMessage, Status>> + Send>>;

    /// Returns detailed coordinator status information.
    ///
    /// Admin-only endpoint that provides system metrics and operational status.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming StatusInfoRequest gRPC request.
    async fn status_info(
        &self,
        request: Request<StatusInfoRequest>,
    ) -> Result<Response<StatusInfoResponse>, Status> {
        self.validate_admin_request(&request)?;

        let status_info = self.coordinator_service.handle_status_info().await;

        Ok(Response::new(status_info.into()))
    }

    /// Basic health check endpoint for service monitoring.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming HealthCheckRequest gRPC request.
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {}))
    }

    /// Returns list of all jobs with their current status.
    ///
    /// Admin-only endpoint for job monitoring and management.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming JobsListRequest gRPC request.
    async fn jobs_list(
        &self,
        request: Request<JobsListRequest>,
    ) -> Result<Response<JobsListResponse>, Status> {
        self.validate_admin_request(&request)?;

        let jobs_list = self.coordinator_service.handle_jobs_list().await;

        Ok(Response::new(jobs_list.into()))
    }

    /// Returns list of all registered workers and their states.
    ///
    /// Admin-only endpoint for worker fleet monitoring.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming WorkersListRequest gRPC request.
    async fn workers_list(
        &self,
        request: Request<WorkersListRequest>,
    ) -> Result<Response<WorkersListResponse>, Status> {
        self.validate_admin_request(&request)?;

        let workers_list = self.coordinator_service.handle_workers_list().await;

        Ok(Response::new(workers_list.into()))
    }

    /// Returns detailed status for a specific job.
    ///
    /// Admin-only endpoint for job inspection and debugging.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming JobStatusRequest gRPC request.
    async fn job_status(
        &self,
        request: Request<JobStatusRequest>,
    ) -> Result<Response<JobStatusResponse>, Status> {
        self.validate_admin_request(&request)?;

        let job_id = JobId::from(request.into_inner().job_id);
        self.coordinator_service
            .handle_job_status(&job_id)
            .await
            .map(|status_dto| Response::new(status_dto.into()))
            .map_err(Status::from)
    }

    /// Returns overall system status and health metrics.
    ///
    /// Admin-only endpoint for system monitoring and alerting.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming SystemStatusRequest gRPC request.
    async fn system_status(
        &self,
        request: Request<SystemStatusRequest>,
    ) -> Result<Response<SystemStatusResponse>, Status> {
        self.validate_admin_request(&request)?;

        let system_status = self.coordinator_service.handle_system_status().await;

        Ok(Response::new(system_status.into()))
    }

    /// Starts a new proof generation job.
    ///
    /// Admin-only endpoint for initiating distributed proof computation.
    ///
    /// # Parameters
    ///
    /// * `request` - The incoming LaunchProofRequest gRPC request.
    async fn launch_proof(
        &self,
        request: Request<LaunchProofRequest>,
    ) -> Result<Response<LaunchProofResponse>, Status> {
        self.validate_admin_request(&request)?;

        let launch_proof_request_dto = request.into_inner().into();
        let result = self.coordinator_service.launch_proof(launch_proof_request_dto).await;

        result.map(|response_dto| Response::new(response_dto.into())).map_err(Status::from)
    }

    /// Bidirectional streaming endpoint for worker communication.
    async fn worker_stream(
        &self,
        request: Request<Streaming<WorkerMessage>>,
    ) -> Result<Response<Self::WorkerStreamStream>, Status> {
        let coordinator_service = self.coordinator_service.clone();
        let mut in_stream = request.into_inner();

        let response_stream = Box::pin(stream! {
            // Create a channel for outbound messages to this worker (for backpressure)
            // The sender will be held by GrpcMessageSender and used by CoordinatorService
            let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<CoordinatorMessage>();
            let grpc_msg_tx = Box::new(GrpcMessageSender::new(outbound_tx));

            // Clean registration handling - wait for worker to introduce itself
            let worker_id = match in_stream.next().await {
                Some(Ok(WorkerMessage { payload: Some(worker_message::Payload::Register(req)) })) => {
                    let requested_worker_id = WorkerId::from(req.worker_id.clone());
                    let (accepted, message) = coordinator_service.handle_stream_registration(req.into(), grpc_msg_tx).await;

                    if accepted {
                        yield Self::registration_response(&requested_worker_id, accepted, message);
                        requested_worker_id
                    } else {
                        yield Self::registration_response(&requested_worker_id, accepted, message);
                        return;
                    }
                }
                Some(Ok(WorkerMessage { payload: Some(worker_message::Payload::Reconnect(req)) })) => {
                    let requested_worker_id = WorkerId::from(req.worker_id.clone());
                    let (accepted, message) = coordinator_service.handle_stream_reconnection(req.into(), grpc_msg_tx).await;

                    if accepted {
                        yield Self::registration_response(&requested_worker_id, accepted, message);
                        requested_worker_id
                    } else {
                        yield Self::registration_response(&requested_worker_id, accepted, message);
                        return;
                    }
                }
                Some(Ok(_)) => {
                    // First message was not registration or reconnection
                    yield Err(Status::invalid_argument("First message must be registration or reconnection"));
                    return;
                }
                Some(Err(e)) => {
                    error!("Error receiving first message: {e}");
                    yield Err(e);
                    return;
                }
                None => {
                    error!("Stream closed without registration");
                    yield Err(Status::aborted("Connection closed during handshake"));
                    return;
                }
            };

            info!("Worker {} registered successfully", worker_id);

            // Now handle the rest of the stream messages
            loop {
                tokio::select! {
                    // Handle incoming messages from worker
                    incoming_result = in_stream.next() => {
                        match incoming_result {
                            Some(Ok(message)) => {
                                if let Err(e) = Self::handle_stream_message(&coordinator_service, &worker_id, message).await {
                                    error!("Error handling worker message: {}", e);
                                    yield Err(Status::from(e));
                                    break;
                                }
                            }
                            Some(Err(e)) => {
                                error!("Error receiving message from worker {worker_id}: {e}");
                                yield Err(e);
                                break;
                            }
                            None => {
                                info!("Worker {} stream ended", worker_id);
                                break;
                            }
                        }
                    }
                    // Handle outgoing messages to worker
                    outbound_result = outbound_rx.recv() => {
                        match outbound_result {
                            Some(message) => {
                                yield Ok(message);
                            }
                            None => {
                                // Channel closed, likely service shutdown
                                info!("Outbound channel closed for worker {}", worker_id);
                                break; // Break out of the loop, ending the stream naturally
                            }
                        }
                    }
                }
            }

            // Stream cleanup - this runs when the loop breaks
            info!("Cleaning up worker {} connection", worker_id);

            // Perform async cleanup
            if let Err(e) = coordinator_service.unregister_worker(&worker_id).await {
                error!("Failed to handle disconnect for worker {}: {}", worker_id, e);
            }
        });

        Ok(Response::new(response_stream))
    }
}
