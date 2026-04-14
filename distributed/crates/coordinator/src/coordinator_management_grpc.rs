use std::sync::Arc;

use async_stream::stream;
use tokio::time::{timeout, Duration};
use tonic::{Request, Response, Status};
use tracing::warn;
use zisk_distributed_grpc_api::coordinator_api::{
    zisk_coordinator_api_server::ZiskCoordinatorApi, coord_job_event, coord_job_kind_result,
    coord_submit_job_request, CoordCancelJobRequest, CoordCancelJobResponse, CoordCostPerType,
    CoordExecuteResult, CoordExecutionStats, CoordJobEvent, CoordJobEventCancelled,
    CoordJobEventCompleted, CoordJobEventFailed, CoordJobEventProgress, CoordJobEventQueued,
    CoordJobEventStarted, CoordJobEventWaitingForInput, CoordJobKindResult, CoordJobPhase,
    CoordJobResponse, CoordJobStatus, CoordProveResult, CoordPushJobInputRequest,
    CoordRegisterGuestProgramRequest, CoordRegisterGuestProgramResponse, CoordSetupProgramRequest,
    CoordSetupResult, CoordSubmitJobRequest, CoordWaitJobResultRequest, CoordWaitJobResultResponse,
    CoordWatchJobRequest,
};

use crate::{
    coordinator_errors::CoordinatorError,
    job_events::{CoordinatorJobEvent, CoordinatorJobResult},
    Coordinator,
};
use zisk_distributed_common::{
    DataId, HintsModeDto, InputsModeDto, JobId, LaunchProofRequestDto,
};

/// gRPC server implementing the internal `ZiskCoordinatorApi` service.
///
/// Exposes job submission, status polling, event streaming, and cancellation
/// to internal clients (primarily the gateway).
pub struct CoordinatorManagementGrpc {
    coordinator: Arc<Coordinator>,
}

impl CoordinatorManagementGrpc {
    pub fn new(coordinator: Arc<Coordinator>) -> Self {
        Self { coordinator }
    }

    fn map_status(err: CoordinatorError) -> Status {
        Status::from(err)
    }
}

fn now_timestamp() -> prost_types::Timestamp {
    let now = std::time::SystemTime::now();
    prost_types::Timestamp::from(now)
}

fn coord_job_phase_from_event_phase(phase: &zisk_distributed_common::JobPhase) -> CoordJobPhase {
    match phase {
        zisk_distributed_common::JobPhase::Contributions
        | zisk_distributed_common::JobPhase::ContributionsInputsStream
        | zisk_distributed_common::JobPhase::ContributionsHintsStream
        | zisk_distributed_common::JobPhase::Execution => CoordJobPhase::Contributions,
        zisk_distributed_common::JobPhase::Prove => CoordJobPhase::Prove,
        zisk_distributed_common::JobPhase::Aggregate => CoordJobPhase::Aggregate,
    }
}

fn build_kind_result(result: CoordinatorJobResult) -> CoordJobKindResult {
    match result {
        CoordinatorJobResult::Setup => CoordJobKindResult {
            result: Some(coord_job_kind_result::Result::Setup(CoordSetupResult {})),
        },
        CoordinatorJobResult::Prove { proof_bytes, stats } => CoordJobKindResult {
            result: Some(coord_job_kind_result::Result::Prove(CoordProveResult {
                proof_data: proof_bytes,
                stats: Some(CoordExecutionStats {
                    steps: stats.steps,
                    duration_nanos: stats.duration_nanos,
                    cost_per_type: Some(CoordCostPerType {
                        main: stats.main_cost,
                        opcode: stats.opcode_cost,
                        memory: stats.memory_cost,
                        precompile: stats.precompile_cost,
                        tables: stats.tables_cost,
                        other: stats.other_cost,
                    }),
                }),
            })),
        },
        CoordinatorJobResult::Execute { stats, public_outputs } => CoordJobKindResult {
            result: Some(coord_job_kind_result::Result::Execute(CoordExecuteResult {
                public_outputs,
                stats: Some(CoordExecutionStats {
                    steps: stats.steps,
                    duration_nanos: stats.duration_nanos,
                    cost_per_type: Some(CoordCostPerType {
                        main: stats.main_cost,
                        opcode: stats.opcode_cost,
                        memory: stats.memory_cost,
                        precompile: stats.precompile_cost,
                        tables: stats.tables_cost,
                        other: stats.other_cost,
                    }),
                }),
            })),
        },
        CoordinatorJobResult::Wrap { proof_bytes } => CoordJobKindResult {
            result: Some(coord_job_kind_result::Result::Wrap(
                zisk_distributed_grpc_api::coordinator_api::CoordWrapResult {
                    proof_data: proof_bytes,
                },
            )),
        },
    }
}

fn coordinator_event_to_grpc(job_id: &str, event: CoordinatorJobEvent) -> CoordJobEvent {
    let ts = Some(now_timestamp());
    let job_id = job_id.to_string();
    match event {
        CoordinatorJobEvent::Queued => CoordJobEvent {
            event: Some(coord_job_event::Event::Queued(CoordJobEventQueued {
                job_id,
                timestamp: ts,
            })),
        },
        CoordinatorJobEvent::Started => CoordJobEvent {
            event: Some(coord_job_event::Event::Started(CoordJobEventStarted {
                job_id,
                timestamp: ts,
            })),
        },
        CoordinatorJobEvent::Progress(phase) => CoordJobEvent {
            event: Some(coord_job_event::Event::Progress(CoordJobEventProgress {
                job_id,
                phase: coord_job_phase_from_event_phase(&phase) as i32,
                timestamp: ts,
            })),
        },
        CoordinatorJobEvent::WaitingForInput => CoordJobEvent {
            event: Some(coord_job_event::Event::WaitingForInput(
                CoordJobEventWaitingForInput { job_id, timestamp: ts },
            )),
        },
        CoordinatorJobEvent::Completed(result) => CoordJobEvent {
            event: Some(coord_job_event::Event::Completed(CoordJobEventCompleted {
                job_id,
                result: Some(build_kind_result(result)),
                timestamp: ts,
            })),
        },
        CoordinatorJobEvent::Failed(reason) => CoordJobEvent {
            event: Some(coord_job_event::Event::Failed(CoordJobEventFailed {
                job_id,
                reason,
                timestamp: ts,
            })),
        },
        CoordinatorJobEvent::Cancelled => CoordJobEvent {
            event: Some(coord_job_event::Event::Cancelled(CoordJobEventCancelled {
                job_id,
                timestamp: ts,
            })),
        },
    }
}

fn is_terminal_event(event: &CoordJobEvent) -> bool {
    matches!(
        event.event,
        Some(coord_job_event::Event::Completed(_))
            | Some(coord_job_event::Event::Failed(_))
            | Some(coord_job_event::Event::Cancelled(_))
    )
}


#[tonic::async_trait]
impl ZiskCoordinatorApi for CoordinatorManagementGrpc {
    type WatchJobStream =
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<CoordJobEvent, Status>> + Send>>;

    async fn register_guest_program(
        &self,
        request: Request<CoordRegisterGuestProgramRequest>,
    ) -> Result<Response<CoordRegisterGuestProgramResponse>, Status> {
        let req = request.into_inner();
        let hash_id = self
            .coordinator
            .register_guest_program(req.elf_bytes)
            .map_err(Self::map_status)?;
        Ok(Response::new(CoordRegisterGuestProgramResponse { hash_id }))
    }

    async fn setup_program(
        &self,
        request: Request<CoordSetupProgramRequest>,
    ) -> Result<Response<CoordJobResponse>, Status> {
        let req = request.into_inner();
        let job_id = self
            .coordinator
            .setup_program(&req.hash_id)
            .await
            .map_err(Self::map_status)?;
        Ok(Response::new(CoordJobResponse { job_id: job_id.as_string() }))
    }

    async fn submit_job(
        &self,
        request: Request<CoordSubmitJobRequest>,
    ) -> Result<Response<CoordJobResponse>, Status> {
        let req = request.into_inner();

        let (inputs_mode, hints_mode, execution_only) = match req.job_kind {
            Some(coord_submit_job_request::JobKind::Prove(ref prove)) => {
                let input_mode = prove
                    .input
                    .as_ref()
                    .and_then(|i| i.source.as_ref())
                    .map(|s| match s {
                        zisk_distributed_grpc_api::coordinator_api::coord_input_kind::Source::Inline(chunk) => {
                            InputsModeDto::InputsData(hex::encode(&chunk.data))
                        }
                        zisk_distributed_grpc_api::coordinator_api::coord_input_kind::Source::StreamUri(uri) => {
                            InputsModeDto::InputsPath(uri.clone())
                        }
                    })
                    .unwrap_or(InputsModeDto::InputsNone);
                (input_mode, HintsModeDto::HintsNone, false)
            }
            Some(coord_submit_job_request::JobKind::Execute(ref exec)) => {
                let input_mode = exec
                    .input
                    .as_ref()
                    .and_then(|i| i.source.as_ref())
                    .map(|s| match s {
                        zisk_distributed_grpc_api::coordinator_api::coord_input_kind::Source::Inline(chunk) => {
                            InputsModeDto::InputsData(hex::encode(&chunk.data))
                        }
                        zisk_distributed_grpc_api::coordinator_api::coord_input_kind::Source::StreamUri(uri) => {
                            InputsModeDto::InputsPath(uri.clone())
                        }
                    })
                    .unwrap_or(InputsModeDto::InputsNone);
                (input_mode, HintsModeDto::HintsNone, true)
            }
            Some(coord_submit_job_request::JobKind::Wrap(_)) => {
                return Err(Status::unimplemented("Wrap jobs are not yet supported"));
            }
            None => return Err(Status::invalid_argument("job_kind is required")),
        };

        let request_dto = LaunchProofRequestDto {
            data_id: DataId::new(),
            compute_capacity: 1,
            minimal_compute_capacity: 1,
            inputs_mode,
            hints_mode,
            simulated_node: None,
            metadata: Default::default(),
            execution_only,
        };

        let response = self
            .coordinator
            .launch_proof(request_dto)
            .await
            .map_err(Self::map_status)?;

        Ok(Response::new(CoordJobResponse { job_id: response.job_id.as_string() }))
    }

    async fn wait_job_result(
        &self,
        request: Request<CoordWaitJobResultRequest>,
    ) -> Result<Response<CoordWaitJobResultResponse>, Status> {
        let req = request.into_inner();
        let job_id = JobId::from(req.job_id.clone());
        let timeout_secs = req.timeout_seconds.unwrap_or(5).max(1) as u64;

        // Subscribe first, then check current state to avoid TOCTOU race
        let mut rx = self
            .coordinator
            .subscribe_job_events(&job_id)
            .await
            .ok_or_else(|| Status::not_found(format!("Job {} not found", req.job_id)))?;

        // Poll for a terminal event with the configured timeout
        let deadline = Duration::from_secs(timeout_secs);
        let result = timeout(deadline, async {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if matches!(
                            event,
                            CoordinatorJobEvent::Completed(_)
                                | CoordinatorJobEvent::Failed(_)
                                | CoordinatorJobEvent::Cancelled
                        ) {
                            return Some(event);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WaitJobResult lagged {} events for job {}", n, req.job_id);
                    }
                }
            }
        })
        .await;

        match result {
            Ok(Some(CoordinatorJobEvent::Completed(job_result))) => {
                Ok(Response::new(CoordWaitJobResultResponse {
                    job_id: req.job_id,
                    job_status: CoordJobStatus::Completed as i32,
                    result: Some(build_kind_result(job_result)),
                }))
            }
            Ok(Some(CoordinatorJobEvent::Failed(_))) => {
                Ok(Response::new(CoordWaitJobResultResponse {
                    job_id: req.job_id,
                    job_status: CoordJobStatus::Failed as i32,
                    result: None,
                }))
            }
            Ok(Some(CoordinatorJobEvent::Cancelled)) => {
                Ok(Response::new(CoordWaitJobResultResponse {
                    job_id: req.job_id,
                    job_status: CoordJobStatus::Cancelled as i32,
                    result: None,
                }))
            }
            Ok(Some(_)) | Ok(None) | Err(_) => {
                // Timeout or channel closed — return current running status
                Ok(Response::new(CoordWaitJobResultResponse {
                    job_id: req.job_id,
                    job_status: CoordJobStatus::Running as i32,
                    result: None,
                }))
            }
        }
    }

    async fn watch_job(
        &self,
        request: Request<CoordWatchJobRequest>,
    ) -> Result<Response<Self::WatchJobStream>, Status> {
        let req = request.into_inner();
        let job_id_str = req.job_id.clone();
        let job_id = JobId::from(req.job_id);

        let rx = self
            .coordinator
            .subscribe_job_events(&job_id)
            .await
            .ok_or_else(|| Status::not_found(format!("Job {} not found", job_id_str)))?;

        let s = stream! {
            let mut rx = rx;
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let grpc_event = coordinator_event_to_grpc(&job_id_str, event);
                        let terminal = is_terminal_event(&grpc_event);
                        yield Ok(grpc_event);
                        if terminal {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WatchJob lagged {} events for job {}", n, job_id_str);
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(s)))
    }

    async fn cancel_job(
        &self,
        request: Request<CoordCancelJobRequest>,
    ) -> Result<Response<CoordCancelJobResponse>, Status> {
        let req = request.into_inner();
        let job_id = JobId::from(req.job_id.clone());

        let cancelled =
            self.coordinator.cancel_job(&job_id).await.map_err(Self::map_status)?;

        Ok(Response::new(CoordCancelJobResponse {
            job_id: req.job_id,
            cancelled,
        }))
    }

    async fn push_job_input(
        &self,
        _request: Request<tonic::Streaming<CoordPushJobInputRequest>>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("PushJobInput is not yet implemented"))
    }
}
