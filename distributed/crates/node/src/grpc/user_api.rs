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

fn coordinator_program_to_summary(p: coord::ProgramInfo) -> GuestProgramSummary {
    GuestProgramSummary {
        program_id: p.program_id,
        hash_id: p.hash_id,
        name: p.name,
        description: p.description,
        author: p.author,
        metadata: p.metadata,
        created_at: p.created_at,
        status: p.status,
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
            available_setups: vec![],
        }))
    }

    // ── Programs ──────────────────────────────────────────────────────────────

    async fn list_guest_programs(
        &self,
        _request: Request<ListGuestProgramsRequest>,
    ) -> Result<Response<ListGuestProgramsResponse>, Status> {
        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;

        let resp = client
            .list_programs(coord::ListProgramsRequest {})
            .await?
            .into_inner();

        let programs = resp.programs.into_iter().map(coordinator_program_to_summary).collect();
        Ok(Response::new(ListGuestProgramsResponse { programs }))
    }

    async fn get_guest_program(
        &self,
        request: Request<GetGuestProgramRequest>,
    ) -> Result<Response<GuestProgramSummary>, Status> {
        use crate::grpc::user::get_guest_program_request::Lookup as UserLookup;
        use coord::get_program_request::Lookup as CoordLookup;

        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;

        let lookup = match request.into_inner().lookup {
            Some(UserLookup::ProgramId(v)) => CoordLookup::ProgramId(v),
            Some(UserLookup::HashId(v)) => CoordLookup::HashId(v),
            Some(UserLookup::Name(v)) => CoordLookup::Name(v),
            None => return Err(Status::invalid_argument("lookup field is required")),
        };

        let resp = client
            .get_program(coord::GetProgramRequest { lookup: Some(lookup) })
            .await?
            .into_inner();

        match resp.program {
            Some(p) => Ok(Response::new(coordinator_program_to_summary(p))),
            None => Err(Status::not_found("program not found")),
        }
    }

    async fn wait_guest_program(
        &self,
        request: Request<WaitGuestProgramRequest>,
    ) -> Result<Response<GuestProgramSummary>, Status> {
        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;
        let program_id = request.into_inner().program_id;

        let resp = client
            .wait_program(coord::WaitProgramRequest { program_id })
            .await?
            .into_inner();

        match resp.program {
            Some(p) => Ok(Response::new(coordinator_program_to_summary(p))),
            None => Err(Status::not_found("program not found")),
        }
    }

    async fn register_guest_program(
        &self,
        request: Request<RegisterGuestProgramRequest>,
    ) -> Result<Response<RegisterGuestProgramResponse>, Status> {
        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;
        let req = request.into_inner();

        let resp = client
            .register_program(coord::RegisterProgramRequest {
                name: req.name,
                description: req.description,
                author: req.author,
                zisk_elf: req.zisk_elf,
                metadata: req.metadata,
            })
            .await?
            .into_inner();

        Ok(Response::new(RegisterGuestProgramResponse {
            hash_id: resp.hash_id,
            program_id: resp.program_id,
            status: resp.status,
        }))
    }

    async fn update_guest_program(
        &self,
        request: Request<UpdateGuestProgramRequest>,
    ) -> Result<Response<UpdateGuestProgramResponse>, Status> {
        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;
        let req = request.into_inner();

        let resp = client
            .update_program(coord::UpdateProgramRequest {
                program_id: req.program_id,
                name: req.name,
                description: req.description,
                author: req.author,
                metadata: req.metadata,
                zisk_elf: req.zisk_elf,
            })
            .await?
            .into_inner();

        Ok(Response::new(UpdateGuestProgramResponse {
            program_id: resp.program_id,
            hash_id: resp.hash_id,
            status: resp.status,
        }))
    }

    async fn delete_guest_program(
        &self,
        request: Request<DeleteGuestProgramRequest>,
    ) -> Result<Response<()>, Status> {
        use crate::grpc::user::delete_guest_program_request::Lookup as UserLookup;
        use coord::delete_program_request::Lookup as CoordLookup;

        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;

        let lookup = match request.into_inner().lookup {
            Some(UserLookup::ProgramId(v)) => CoordLookup::ProgramId(v),
            Some(UserLookup::HashId(v)) => CoordLookup::HashId(v),
            None => return Err(Status::invalid_argument("lookup field is required")),
        };

        client
            .delete_program(coord::DeleteProgramRequest { lookup: Some(lookup) })
            .await?;

        Ok(Response::new(()))
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
        let coordinator = self.state.coordinator.as_ref().ok_or_else(Self::coordinator_unavailable)?;
        let mut client = coordinator.lock().await;
        let job_id = request.into_inner().job_id;

        let resp = client
            .wait_job(coord::WaitJobRequest { job_id })
            .await
            .map_err(|e| Status::internal(e.message()))?
            .into_inner();

        match resp.result {
            Some(coord::job_status_response::Result::Job(j)) => Ok(Response::new(coordinator_job_to_info(j))),
            Some(coord::job_status_response::Result::Error(e)) => Err(map_coordinator_error(e)),
            None => Err(Status::internal("empty response from coordinator")),
        }
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
