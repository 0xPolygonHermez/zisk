//! gRPC transport layer for the distributed proof coordination system.
//!
//! This module provides the gRPC server implementation that exposes the coordinator's
//! functionality over the network, handling bidirectional streaming communication
//! with workers and admin API endpoints.

use async_stream::stream;
use futures_util::{Stream, StreamExt};
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info};
use zisk_cluster_api::{zisk_distributed_api_server::*, *};
use zisk_cluster_common::{CoordinatorMessageDto, SetupProgramAckDto, WorkerId};

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

/// Simple drop guard that automatically disconnects worker when dropped.
/// Uses connection generation to prevent stale guards from undoing reconnections.
struct ConnectionDropGuard {
    worker_id: WorkerId,
    coordinator: Arc<Coordinator>,
    generation: u64,
}

impl ConnectionDropGuard {
    fn new(worker_id: WorkerId, coordinator: Arc<Coordinator>, generation: u64) -> Self {
        Self { worker_id, coordinator, generation }
    }
}

impl Drop for ConnectionDropGuard {
    fn drop(&mut self) {
        let worker_id = self.worker_id.clone();
        let coordinator = self.coordinator.clone();
        let generation = self.generation;

        // Spawn async cleanup on drop. Sleep for the reconnection grace period first so
        // a transient network blip does not kill the job. The generation check inside
        // disconnect_worker_if_generation is a no-op if the worker reconnected (and thus
        // incremented its connection generation) before the timer fires.
        let grace_ms = coordinator.config().coordinator.reconnect_grace_period_ms;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(grace_ms)).await;
            if let Err(e) =
                coordinator.disconnect_worker_if_generation(&worker_id, generation).await
            {
                error!("Failed to disconnect worker {}: {}", worker_id, e);
            }
        });
    }
}

/// gRPC server implementation for the distributed proof coordination system.
///
/// Serves as the network transport layer, exposing coordinator functionality through:
/// - Admin API endpoints (status, job management, system monitoring)  
/// - Bidirectional streaming for worker communication
/// - Authentication and authorization for admin endpoints
pub struct CoordinatorGrpc {
    coordinator: Arc<Coordinator>,
    _monitor_handle: tokio::task::JoinHandle<()>,
}

impl CoordinatorGrpc {
    /// Creates a new gRPC service instance with the given configuration.
    ///
    /// # Parameters
    ///
    /// * `config` - Configuration parameters for the coordinator service.
    pub async fn new(config: Config) -> CoordinatorResult<Self> {
        let coordinator = Arc::new(Coordinator::new(config));
        let monitor_handle = coordinator.start_job_monitor();
        Ok(Self { coordinator, _monitor_handle: monitor_handle })
    }

    /// Creates a gRPC service from a pre-built shared `Arc<Coordinator>`.
    ///
    /// Use this when multiple services need to share the same coordinator instance.
    pub fn from_arc(coordinator: Arc<Coordinator>) -> Self {
        let monitor_handle = coordinator.start_job_monitor();
        Self { coordinator, _monitor_handle: monitor_handle }
    }

    /// Returns a clone of the underlying `Arc<Coordinator>`.
    pub fn coordinator(&self) -> Arc<Coordinator> {
        Arc::clone(&self.coordinator)
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
                worker_message::Payload::JobCancelledAck(ack) => {
                    Self::validate_same_worker_id(worker_id, &ack.worker_id)?;
                    coordinator.handle_stream_job_cancelled_ack(worker_id, &ack.job_id.into()).await
                }
                worker_message::Payload::SetupProgramAck(ack) => {
                    Self::validate_same_worker_id(worker_id, &ack.worker_id)?;
                    coordinator
                        .handle_stream_setup_program_ack(SetupProgramAckDto {
                            job_id: ack.job_id,
                            worker_id: worker_id.clone(),
                            hash_id: ack.hash_id,
                            success: ack.success,
                            error_message: if ack.error_message.is_empty() {
                                None
                            } else {
                                Some(ack.error_message)
                            },
                            vk: ack.vk,
                        })
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
        directive: Option<ReconnectionDirective>,
        setup_program: Option<SetupProgram>,
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
                directive,
                setup_program,
            })),
        })
    }
}

/// Implements the gRPC service trait generated by tonic from the protobuf definitions.
#[tonic::async_trait]
impl ZiskDistributedApi for CoordinatorGrpc {
    type WorkerStreamStream =
        Pin<Box<dyn Stream<Item = Result<CoordinatorMessage, Status>> + Send>>;

    /// Bidirectional streaming endpoint for worker communication.
    async fn worker_stream(
        &self,
        request: Request<Streaming<WorkerMessage>>,
    ) -> Result<Response<Self::WorkerStreamStream>, Status> {
        let coordinator = self.coordinator.clone();
        let mut in_stream = request.into_inner();

        let response_stream = Box::pin(stream! {
            // Create a channel for outbound messages to this worker (for backpressure)
            let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<CoordinatorMessage>();
            let grpc_msg_tx = Box::new(GrpcMessageSender::new(outbound_tx));

            // Clean registration handling - wait for worker to introduce itself
            let worker_id = match in_stream.next().await {
                Some(Ok(WorkerMessage { payload: Some(worker_message::Payload::Register(req)) })) => {
                    let req_worker_id = WorkerId::from(req.worker_id.clone());
                    let (accepted, message, setup) =
                        coordinator.handle_stream_registration(req.into(), grpc_msg_tx).await;

                        yield Self::registration_response(&req_worker_id, accepted, message, None, setup.map(Into::into));

                    if !accepted { return; }

                    req_worker_id
                }
                Some(Ok(WorkerMessage { payload: Some(worker_message::Payload::Reconnect(req)) })) => {
                    let req_worker_id = WorkerId::from(req.worker_id.clone());
                    let (accepted, message, directive, setup) =
                        coordinator.handle_stream_reconnection(req.into(), grpc_msg_tx).await;

                    yield Self::registration_response(
                        &req_worker_id,
                        accepted,
                        message,
                        directive.map(Into::into),
                        setup.map(Into::into),
                    );

                    if !accepted { return; }

                    req_worker_id
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

            info!("{worker_id} registered successfully");

            // Get the connection generation to pass to the guard
            let generation = coordinator
                .workers_pool()
                .connection_generation(&worker_id)
                .await
                .unwrap_or(0);

            // Drop guard handles ALL cleanup — uses generation to avoid race with reconnection
            let _guard = ConnectionDropGuard::new(worker_id.clone(), coordinator.clone(), generation);

            // Main stream loop
            loop {
                tokio::select! {
                    // Incoming worker messages
                    incoming = in_stream.next() => {
                        match incoming {
                            Some(Ok(message)) => {
                                if let Err(e) = Self::handle_stream_message(&coordinator, &worker_id, message).await {
                                    error!("Error handling worker {worker_id}: {e}");
                                }
                            }
                            Some(Err(e)) => {
                                error!("Error receiving from {worker_id}: {e}");
                                yield Err(e);
                                return; // guard handles cleanup
                            }
                            None => {
                                info!("Worker {worker_id} disconnected");
                                return; // guard handles cleanup
                            }
                        }
                    }

                    // Outbound messages to worker
                    outbound = outbound_rx.recv() => {
                        match outbound {
                            Some(message) => yield Ok(message),
                            None => {
                                // Channel closed, likely service shutdown
                                info!("Outbound channel closed for worker {}", worker_id);
                                return; // guard handles cleanup
                            }
                        }
                    }
                }
            }
        });

        Ok(Response::new(response_stream))
    }
}
