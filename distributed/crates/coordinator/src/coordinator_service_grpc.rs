use anyhow::Result;
use async_stream::stream;
use distributed_common::{CoordinatorMessageDto, JobId, ProverId};
use distributed_grpc_api::{distributed_api_server::*, *};
use futures_util::{Stream, StreamExt};
use std::{pin::Pin, sync::Arc};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info};

use crate::config::Config;
use crate::coordinator_service::MessageSender;
use crate::CoordinatorService;

/// Wrapper around mpsc::UnboundedSender to implement MessageSender trait for gRPC
/// and decouple gRPC stream from CoordinatorService
pub struct GrpcMessageSender(mpsc::UnboundedSender<CoordinatorMessage>);

impl GrpcMessageSender {
    pub fn new(sender: mpsc::UnboundedSender<CoordinatorMessage>) -> Self {
        Self(sender)
    }
}

impl MessageSender for GrpcMessageSender {
    fn send(&self, msg: CoordinatorMessageDto) -> Result<()> {
        self.0.send(msg.into())?;
        Ok(())
    }
}

/// gRPC transport layer implementation for the distributed proof coordination system.
///
/// This struct serves as the gRPC adapter that bridges external network communication
/// with the core coordinator business logic.
pub struct CoordinatorServiceGrpc {
    coordinator_service: Arc<CoordinatorService>,
}

impl CoordinatorServiceGrpc {
    pub async fn new(config: Config) -> Result<Self> {
        Ok(Self { coordinator_service: Arc::new(CoordinatorService::new(config)) })
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

    fn validate_same_prover_id(prover_id: &ProverId, request_prover_id: &str) -> Result<()> {
        anyhow::ensure!(
            prover_id.as_string() == request_prover_id,
            "Prover ID mismatch: expected {}, got {}",
            prover_id.as_string(),
            request_prover_id
        );
        Ok(())
    }

    async fn handle_stream_message(
        coordinator: &CoordinatorService,
        prover_id: &ProverId,
        message: ProverMessage,
    ) -> Result<()> {
        match message.payload {
            Some(payload) => match payload {
                prover_message::Payload::HeartbeatAck(heartbeat_ack) => {
                    Self::validate_same_prover_id(prover_id, &heartbeat_ack.prover_id)?;
                    coordinator.handle_stream_heartbeat_ack(heartbeat_ack.into()).await
                }
                prover_message::Payload::Error(prover_error) => {
                    Self::validate_same_prover_id(prover_id, &prover_error.prover_id)?;
                    coordinator.handle_stream_error(prover_error.into()).await
                }
                prover_message::Payload::Register(prover_register_req) => {
                    Self::validate_same_prover_id(prover_id, &prover_register_req.prover_id)?;
                    coordinator.handle_stream_register(prover_register_req.into()).await
                }
                prover_message::Payload::Reconnect(prover_reconnect_req) => {
                    Self::validate_same_prover_id(prover_id, &prover_reconnect_req.prover_id)?;
                    coordinator.handle_stream_reconnect(prover_reconnect_req.into()).await
                }
                prover_message::Payload::ExecuteTaskResponse(execute_task_response) => {
                    Self::validate_same_prover_id(prover_id, &execute_task_response.prover_id)?;
                    coordinator
                        .handle_stream_execute_task_response(execute_task_response.into())
                        .await
                }
            },
            None => {
                Err(anyhow::anyhow!("Received message with no payload from prover {}", prover_id))
            }
        }
    }

    fn registration_response(
        prover_id: &ProverId,
        accepted: bool,
        message: String,
    ) -> Result<CoordinatorMessage, Status> {
        Ok(CoordinatorMessage {
            payload: Some(coordinator_message::Payload::RegisterResponse(ProverRegisterResponse {
                prover_id: prover_id.as_string(),
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

#[tonic::async_trait]
impl DistributedApi for CoordinatorServiceGrpc {
    type ProverStreamStream =
        Pin<Box<dyn Stream<Item = Result<CoordinatorMessage, Status>> + Send>>;

    async fn status_info(
        &self,
        request: Request<StatusInfoRequest>,
    ) -> Result<Response<StatusInfoResponse>, Status> {
        self.validate_admin_request(&request)?;

        let status_info = self.coordinator_service.handle_status_info().await;

        Ok(Response::new(status_info.into()))
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {}))
    }

    async fn jobs_list(
        &self,
        request: Request<JobsListRequest>,
    ) -> Result<Response<JobsListResponse>, Status> {
        self.validate_admin_request(&request)?;

        let jobs_list = self.coordinator_service.handle_jobs_list().await;

        Ok(Response::new(jobs_list.into()))
    }

    async fn provers_list(
        &self,
        request: Request<ProversListRequest>,
    ) -> Result<Response<ProversListResponse>, Status> {
        self.validate_admin_request(&request)?;

        let provers_list = self.coordinator_service.handle_provers_list().await;

        Ok(Response::new(provers_list.into()))
    }

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
            .map_err(|e| Status::internal(format!("Failed to get job status: {}", e)))
    }

    async fn system_status(
        &self,
        request: Request<SystemStatusRequest>,
    ) -> Result<Response<SystemStatusResponse>, Status> {
        self.validate_admin_request(&request)?;

        let system_status = self.coordinator_service.handle_system_status().await;

        Ok(Response::new(system_status.into()))
    }

    async fn launch_proof(
        &self,
        request: Request<LaunchProofRequest>,
    ) -> Result<Response<LaunchProofResponse>, Status> {
        self.validate_admin_request(&request)?;

        let launch_proof_request_dto = request.into_inner().into();
        let result = self.coordinator_service.launch_proof(launch_proof_request_dto).await;

        result
            .map(|response_dto| Response::new(response_dto.into()))
            .map_err(|e| Status::internal(format!("Failed to start proof: {}", e)))
    }

    /// Bidirectional stream for communication between coordinator and provers.
    /// This handles stream messages defined in the .proto file.
    async fn prover_stream(
        &self,
        request: Request<Streaming<ProverMessage>>,
    ) -> Result<Response<Self::ProverStreamStream>, Status> {
        let coordinator_service = self.coordinator_service.clone();
        let mut in_stream = request.into_inner();

        let response_stream = Box::pin(stream! {
            // Create a channel for outbound messages to this prover (for backpressure)
            // The sender will be held by GrpcMessageSender and used by CoordinatorService
            let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<CoordinatorMessage>();
            let grpc_msg_tx = Box::new(GrpcMessageSender::new(outbound_tx));

            // Clean registration handling - wait for prover to introduce itself
            let prover_id = match in_stream.next().await {
                Some(Ok(ProverMessage { payload: Some(prover_message::Payload::Register(req)) })) => {
                    let requested_prover_id = ProverId::from(req.prover_id.clone());
                    let (accepted, message) = coordinator_service.handle_stream_registration(req.into(), grpc_msg_tx).await;

                    if accepted {
                        yield Self::registration_response(&requested_prover_id, accepted, message);
                        requested_prover_id
                    } else {
                        yield Self::registration_response(&requested_prover_id, accepted, message);
                        return;
                    }
                }
                Some(Ok(ProverMessage { payload: Some(prover_message::Payload::Reconnect(req)) })) => {
                    let requested_prover_id = ProverId::from(req.prover_id.clone());
                    let (accepted, message) = coordinator_service.handle_stream_reconnection(req.into(), grpc_msg_tx).await;

                    if accepted {
                        yield Self::registration_response(&requested_prover_id, accepted, message);
                        requested_prover_id
                    } else {
                        yield Self::registration_response(&requested_prover_id, accepted, message);
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

            info!("Prover {} registered successfully, starting message loop", prover_id);

            // Now handle the rest of the stream messages
            loop {
                tokio::select! {
                    // Handle incoming messages from prover
                    incoming_result = in_stream.next() => {
                        match incoming_result {
                            Some(Ok(message)) => {
                                if let Err(e) = Self::handle_stream_message(&coordinator_service, &prover_id, message).await {
                                    error!("Error handling prover message: {}", e);
                                    yield Err(Status::internal(format!("Error handling message: {e}")));
                                    break;
                                }
                            }
                            Some(Err(e)) => {
                                error!("Error receiving message from prover {prover_id}: {e}");
                                yield Err(e);
                                break;
                            }
                            None => {
                                info!("Prover {} stream ended", prover_id);
                                break;
                            }
                        }
                    }
                    // Handle outgoing messages to prover
                    outbound_result = outbound_rx.recv() => {
                        match outbound_result {
                            Some(message) => {
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

            // Perform async cleanup
            if let Err(e) = coordinator_service.unregister_prover(&prover_id).await {
                error!("Failed to handle disconnect for prover {}: {}", prover_id, e);
            }
        });

        Ok(Response::new(response_stream))
    }
}
