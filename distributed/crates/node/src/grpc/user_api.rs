use crate::cluster::ClusterRegistry;
use crate::coordinator::CoordinatorClient;
use crate::grpc::user::zisk_user_api_server::ZiskUserApi;
use crate::grpc::user::*;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};
use zisk_distributed_grpc_api as coord;

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

pub struct UserApiState {
    pub cluster_registry: Option<Arc<ClusterRegistry>>,
    pub coordinator: Option<Mutex<CoordinatorClient>>,
}

impl UserApiState {
    pub fn new(
        cluster_registry: Option<Arc<ClusterRegistry>>,
        coordinator: Option<CoordinatorClient>,
    ) -> Self {
        Self { cluster_registry, coordinator: coordinator.map(Mutex::new) }
    }
}

pub struct UserApiService {
    state: Arc<UserApiState>,
}

impl UserApiService {
    pub fn new(state: Arc<UserApiState>) -> Self {
        Self { state }
    }

    fn coordinator_unavailable() -> Status {
        Status::unavailable("no coordinator configured")
    }
}

// ── Type mapping helpers ──────────────────────────────────────────────────────

fn map_coordinator_job_status(state: &str, phase: &str) -> JobStatus {
    let code = match state {
        "Created" => JobStatusCode::Queued,
        s if s.starts_with("Running") => JobStatusCode::Running,
        "Completed" => JobStatusCode::Completed,
        "Failed" => JobStatusCode::Failed,
        "Cancelled" => JobStatusCode::Cancelled,
        _ => JobStatusCode::JobStatusUnspecified,
    } as i32;

    let phase_val = match phase {
        "Contributions" => JobPhase::Contributions,
        "Prove" => JobPhase::Prove,
        "Aggregate" => JobPhase::Aggregate,
        _ => JobPhase::Contributions,
    } as i32;

    JobStatus { code, phase: phase_val }
}

fn ms_to_timestamp(ms: u64) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: (ms / 1000) as i64,
        nanos: ((ms % 1000) * 1_000_000) as i32,
    }
}

fn coordinator_job_to_summary(j: coord::JobStatus) -> JobSummary {
    JobSummary {
        job_id: j.job_id,
        program_id: j.data_id,
        kind: Some(JobKind { kind: Some(job_kind::Kind::Prove(ProofKind::Stark as i32)) }),
        status: Some(map_coordinator_job_status(&j.state, &j.phase)),
        created_at: Some(ms_to_timestamp(j.start_time)),
    }
}

fn coordinator_job_to_info(j: coord::JobStatus) -> JobInfo {
    let status = map_coordinator_job_status(&j.state, &j.phase);
    let completed_at = if matches!(
        JobStatusCode::try_from(status.code).unwrap_or(JobStatusCode::JobStatusUnspecified),
        JobStatusCode::Completed | JobStatusCode::Failed | JobStatusCode::Cancelled
    ) {
        Some(ms_to_timestamp(j.start_time + j.duration_ms))
    } else {
        None
    };

    JobInfo {
        job_id: j.job_id,
        program_id: j.data_id,
        kind: Some(JobKind { kind: Some(job_kind::Kind::Prove(ProofKind::Stark as i32)) }),
        status: Some(status),
        result: None,
        error: None,
        created_at: Some(ms_to_timestamp(j.start_time)),
        completed_at,
    }
}

fn map_coordinator_error(e: coord::ErrorResponse) -> Status {
    match e.code.as_str() {
        "NOT_FOUND" => Status::not_found(e.message),
        "INVALID_ARGUMENT" => Status::invalid_argument(e.message),
        _ => Status::internal(e.message),
    }
}

// ── ZiskUserApi implementation ────────────────────────────────────────────────

#[tonic::async_trait]
impl ZiskUserApi for UserApiService {
    // ── Node info ─────────────────────────────────────────────────────────────

    async fn get_node_info(
        &self,
        _request: Request<GetNodeInfoRequest>,
    ) -> Result<Response<NodeInfo>, Status> {
        Ok(Response::new(NodeInfo {
            zisk_version: env!("CARGO_PKG_VERSION").to_string(),
            supported_proofs: vec![],
        }))
    }

    // ── Programs ──────────────────────────────────────────────────────────────

    async fn list_guest_programs(
        &self,
        _request: Request<ListGuestProgramsRequest>,
    ) -> Result<Response<ListGuestProgramsResponse>, Status> {
        Err(Status::unimplemented("list_guest_programs not yet implemented"))
    }

    async fn get_guest_program(
        &self,
        _request: Request<GetGuestProgramRequest>,
    ) -> Result<Response<GuestProgramSummary>, Status> {
        Err(Status::unimplemented("get_guest_program not yet implemented"))
    }

    async fn add_guest_program(
        &self,
        _request: Request<AddGuestProgramRequest>,
    ) -> Result<Response<AddGuestProgramResponse>, Status> {
        Err(Status::unimplemented("add_guest_program not yet implemented"))
    }

    async fn update_guest_program(
        &self,
        _request: Request<UpdateGuestProgramRequest>,
    ) -> Result<Response<UpdateGuestProgramResponse>, Status> {
        Err(Status::unimplemented("update_guest_program not yet implemented"))
    }

    async fn delete_guest_program(
        &self,
        _request: Request<DeleteGuestProgramRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("delete_guest_program not yet implemented"))
    }

    // ── Proof jobs ────────────────────────────────────────────────────────────

    type ProveStream = BoxStream<JobEvent>;

    async fn prove(
        &self,
        _request: Request<Streaming<ProveClientMessage>>,
    ) -> Result<Response<Self::ProveStream>, Status> {
        Err(Status::unimplemented("prove not yet implemented"))
    }

    async fn list_jobs(
        &self,
        _request: Request<ListJobsRequest>,
    ) -> Result<Response<ListJobsResponse>, Status> {
        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;

        let resp = client
            .inner
            .jobs_list(coord::JobsListRequest { active_only: false })
            .await
            .map_err(|e| Status::internal(e.message()))?
            .into_inner();

        match resp.result {
            Some(coord::jobs_list_response::Result::JobsList(list)) => {
                let jobs = list.jobs.into_iter().map(coordinator_job_to_summary).collect();
                Ok(Response::new(ListJobsResponse { jobs }))
            }
            Some(coord::jobs_list_response::Result::Error(e)) => Err(map_coordinator_error(e)),
            None => Ok(Response::new(ListJobsResponse { jobs: vec![] })),
        }
    }

    async fn get_job(
        &self,
        request: Request<GetJobRequest>,
    ) -> Result<Response<JobInfo>, Status> {
        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;
        let job_id = request.into_inner().job_id;

        let resp = client
            .inner
            .job_status(coord::JobStatusRequest { job_id })
            .await
            .map_err(|e| Status::internal(e.message()))?
            .into_inner();

        match resp.result {
            Some(coord::job_status_response::Result::Job(j)) => {
                Ok(Response::new(coordinator_job_to_info(j)))
            }
            Some(coord::job_status_response::Result::Error(e)) => Err(map_coordinator_error(e)),
            None => Err(Status::internal("empty response from coordinator")),
        }
    }

    async fn wait_job_result(
        &self,
        request: Request<WaitJobResultRequest>,
    ) -> Result<Response<JobInfo>, Status> {
        let id = request.into_inner().job_id;
        Err(Status::unimplemented(format!("wait_job_result not yet implemented (job '{id}')")))
    }

    async fn push_job_input(
        &self,
        _request: Request<Streaming<PushJobInputRequest>>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("push_job_input not yet implemented"))
    }

    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;
        let job_id = request.into_inner().job_id;

        let resp = client
            .inner
            .cancel_job(coord::CancelJobRequest { job_id: job_id.clone(), reason: None })
            .await
            .map_err(|e| Status::internal(e.message()))?
            .into_inner();

        match resp.result {
            Some(coord::cancel_job_response::Result::JobId(id)) => Ok(Response::new(CancelJobResponse {
                job_id: id,
                job_status: Some(JobStatus {
                    code: JobStatusCode::Cancelled as i32,
                    phase: JobPhase::Contributions as i32,
                }),
            })),
            Some(coord::cancel_job_response::Result::Error(e)) => Err(map_coordinator_error(e)),
            None => Err(Status::internal("empty response from coordinator")),
        }
    }
}
