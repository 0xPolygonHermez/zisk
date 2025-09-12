use async_stream::stream;
use distributed_common::{JobId, ProverId};
use distributed_config::Config;
use distributed_grpc_api::{distributed_api_server::*, *};
use futures_util::{Stream, StreamExt};
use std::sync::atomic::Ordering;
use std::{pin::Pin, sync::Arc};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info};

use crate::CoordinatorService;

/// gRPC Service layer - handles transport and delegates to DistributedService
pub struct CoordinatorServiceGrpc {
    coordinator_service: Arc<CoordinatorService>,
}

impl CoordinatorServiceGrpc {
    pub async fn new(config: Config) -> distributed_common::Result<Self> {
        Ok(Self { coordinator_service: Arc::new(CoordinatorService::new(config).await?) })
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

    async fn handle_stream_message(
        coordinator: &CoordinatorService,
        prover_id: &ProverId,
        message: ProverMessage,
    ) -> anyhow::Result<()> {
        match message.payload.unwrap() {
            prover_message::Payload::HeartbeatAck(update) => {
                coordinator.handle_stream_heartbeat_ack(prover_id, update).await
            }
            prover_message::Payload::Error(result) => {
                coordinator.handle_stream_error(prover_id, result).await
            }
            prover_message::Payload::Register(result) => {
                coordinator.handle_stream_register(prover_id, result).await
            }
            prover_message::Payload::Reconnect(result) => {
                coordinator.handle_stream_reconnect(prover_id, result).await
            }
            prover_message::Payload::ExecuteTaskResponse(result) => {
                coordinator.handle_stream_execute_task_response(prover_id, result).await
            }
        }
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
        let status_info = self.coordinator_service.status_info();

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

        let jobs_list = self.coordinator_service.jobs_list();

        Ok(Response::new(jobs_list.into()))
    }

    async fn provers_list(
        &self,
        request: Request<ProversListRequest>,
    ) -> Result<Response<ProversListResponse>, Status> {
        self.validate_admin_request(&request)?;

        let provers_list = self.coordinator_service.provers_list();

        Ok(Response::new(provers_list.into()))
    }

    async fn job_status(
        &self,
        request: Request<JobStatusRequest>,
    ) -> Result<Response<JobStatusResponse>, Status> {
        self.validate_admin_request(&request)?;

        let job_id = JobId::from(request.get_ref().job_id.clone());
        let job_status = self.coordinator_service.job_status(&job_id);

        Ok(Response::new(job_status.into()))
    }

    async fn system_status(
        &self,
        request: Request<SystemStatusRequest>,
    ) -> Result<Response<SystemStatusResponse>, Status> {
        self.validate_admin_request(&request)?;

        let system_status = self.coordinator_service.handle_system_status().await;
        Ok(Response::new(system_status.into()))
    }

    async fn start_proof(
        &self,
        request: Request<StartProofRequest>,
    ) -> Result<Response<StartProofResponse>, Status> {
        self.validate_admin_request(&request)?;

        let start_proof_request_dto = request.into_inner().into();

        match self.coordinator_service.start_proof(start_proof_request_dto).await {
            Ok(response_dto) => Ok(Response::new(response_dto.into())),
            Err(e) => Err(Status::internal(format!("Failed to start proof: {}", e))),
        }
    }

    /// Bidirectional stream for prover communication
    async fn prover_stream(
        &self,
        request: Request<Streaming<ProverMessage>>,
    ) -> Result<Response<Self::ProverStreamStream>, Status> {
        // Check connection limits first
        let max_connections = self.coordinator_service.max_concurrent_connections() as usize;
        let active_connections = self.coordinator_service.active_connections();

        if active_connections.load(Ordering::SeqCst) >= max_connections as u32 {
            return Err(Status::resource_exhausted(format!(
                "Maximum concurrent connections reached: {}/{}",
                active_connections.load(Ordering::SeqCst),
                max_connections
            )));
        }

        let mut in_stream = request.into_inner();
        let coordinator_service = self.coordinator_service.clone();
        let response_stream = Box::pin(stream! {
            // Increment connection counter
            active_connections.fetch_add(1, Ordering::SeqCst);

            // Create BOUNDED channel for outbound messages to this prover (for backpressure)
            let (outbound_sender, mut outbound_receiver) = mpsc::unbounded_channel::<CoordinatorMessage>();

            // Clean registration handling - wait for prover to introduce itself
            let prover_id = match in_stream.next().await {
                Some(Ok(ProverMessage { payload: Some(prover_message::Payload::Register(req)) })) => {
                    match coordinator_service.handle_stream_registration(req.into(), outbound_sender).await {
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
                    match coordinator_service.handle_stream_reconnection(req.into(), outbound_sender).await {
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
                                if let Err(e) = Self::handle_stream_message(&coordinator_service, &prover_id, message).await {
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
            if let Err(e) = coordinator_service.unregister_prover(&prover_id).await {
                error!("Failed to handle disconnect for prover {}: {}", prover_id, e);
            }
        });

        Ok(Response::new(response_stream))
    }
}
