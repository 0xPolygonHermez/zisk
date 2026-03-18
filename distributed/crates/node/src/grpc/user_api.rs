use crate::cluster::ClusterRegistry;
use crate::grpc::user::zisk_user_api_server::ZiskUserApi;
use crate::grpc::user::*;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

pub struct UserApiState {
    pub cluster_registry: Option<Arc<ClusterRegistry>>,
    // TODO: job_registry: Arc<JobRegistry>,
    // TODO: program_store: Arc<ProgramStore>,
}

impl UserApiState {
    pub fn new(cluster_registry: Option<Arc<ClusterRegistry>>) -> Self {
        Self { cluster_registry }
    }
}

pub struct UserApiService {
    #[allow(dead_code)]
    state: Arc<UserApiState>,
}

impl UserApiService {
    pub fn new(state: Arc<UserApiState>) -> Self {
        Self { state }
    }
}

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
        Ok(Response::new(ListJobsResponse { jobs: vec![] }))
    }

    async fn get_job(
        &self,
        request: Request<GetJobRequest>,
    ) -> Result<Response<JobInfo>, Status> {
        let id = request.into_inner().job_id;
        Err(Status::not_found(format!("job '{id}' not found")))
    }

    async fn wait_job_result(
        &self,
        request: Request<WaitJobResultRequest>,
    ) -> Result<Response<JobInfo>, Status> {
        let id = request.into_inner().job_id;
        Err(Status::not_found(format!("job '{id}' not found")))
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
        let id = request.into_inner().job_id;
        Err(Status::not_found(format!("job '{id}' not found")))
    }
}
