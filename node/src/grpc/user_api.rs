use crate::grpc::user::zisk_user_api_server::ZiskUserApi;
use crate::grpc::user::*;
use crate::service::{NodeService, ProgramLookup, ProgramOrHashLookup};
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

pub struct UserApiService {
    node_service: Arc<NodeService>,
}

impl UserApiService {
    pub fn new(node_service: Arc<NodeService>) -> Self {
        Self { node_service }
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
        let info = self.node_service.get_node_info().await.map_err(Status::from)?;
        Ok(Response::new(info.into()))
    }

    // ── Programs ──────────────────────────────────────────────────────────────

    async fn list_guest_programs(
        &self,
        _request: Request<ListGuestProgramsRequest>,
    ) -> Result<Response<ListGuestProgramsResponse>, Status> {
        let programs = self.node_service.list_programs().await.map_err(Status::from)?;
        Ok(Response::new(ListGuestProgramsResponse {
            programs: programs.into_iter().map(Into::into).collect(),
        }))
    }

    async fn get_guest_program(
        &self,
        request: Request<GetGuestProgramRequest>,
    ) -> Result<Response<GuestProgramSummary>, Status> {
        let lookup = request
            .into_inner()
            .lookup
            .map(ProgramLookup::from)
            .ok_or_else(|| Status::invalid_argument("lookup field is required"))?;

        let program = self.node_service.get_program(lookup).await.map_err(Status::from)?;
        Ok(Response::new(program.into()))
    }

    async fn wait_guest_program(
        &self,
        request: Request<WaitGuestProgramRequest>,
    ) -> Result<Response<GuestProgramSummary>, Status> {
        let program_id = request.into_inner().program_id;
        let program = self.node_service.wait_program(program_id).await.map_err(Status::from)?;
        Ok(Response::new(program.into()))
    }

    async fn register_guest_program(
        &self,
        request: Request<RegisterGuestProgramRequest>,
    ) -> Result<Response<RegisterGuestProgramResponse>, Status> {
        let result = self
            .node_service
            .register_program(request.into_inner().into())
            .await
            .map_err(Status::from)?;
        Ok(Response::new(result.into()))
    }

    async fn update_guest_program(
        &self,
        request: Request<UpdateGuestProgramRequest>,
    ) -> Result<Response<UpdateGuestProgramResponse>, Status> {
        let result = self
            .node_service
            .update_program(request.into_inner().into())
            .await
            .map_err(Status::from)?;
        Ok(Response::new(result.into()))
    }

    async fn delete_guest_program(
        &self,
        request: Request<DeleteGuestProgramRequest>,
    ) -> Result<Response<()>, Status> {
        let lookup = request
            .into_inner()
            .lookup
            .map(ProgramOrHashLookup::from)
            .ok_or_else(|| Status::invalid_argument("lookup field is required"))?;

        self.node_service.delete_program(lookup).await.map_err(Status::from)?;
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
        let jobs = self.node_service.list_jobs().await.map_err(Status::from)?;
        Ok(Response::new(ListJobsResponse { jobs: jobs.into_iter().map(Into::into).collect() }))
    }

    async fn get_job(
        &self,
        request: Request<GetJobRequest>,
    ) -> Result<Response<JobInfo>, Status> {
        let job_id = request.into_inner().job_id;
        let info = self.node_service.get_job(job_id).await.map_err(Status::from)?;
        Ok(Response::new(info.into()))
    }

    async fn wait_job_result(
        &self,
        request: Request<WaitJobResultRequest>,
    ) -> Result<Response<JobInfo>, Status> {
        let job_id = request.into_inner().job_id;
        let info = self.node_service.wait_job(job_id).await.map_err(Status::from)?;
        Ok(Response::new(info.into()))
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
        let job_id = request.into_inner().job_id;
        let result = self.node_service.cancel_job(job_id).await.map_err(Status::from)?;
        Ok(Response::new(result.into()))
    }
}
