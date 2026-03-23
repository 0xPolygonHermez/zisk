use crate::grpc::conversions::coord_state_to_job_event;
use crate::grpc::user::zisk_user_api_server::ZiskUserApi;
use crate::grpc::user::*;
use crate::service::types::{LaunchProofParams, ProofInputSource};
use crate::service::{ProgramLookup, ProgramOrHashLookup, ZiskNodeService};
use async_stream::stream;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};
use zisk_distributed_grpc_api as coord;

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

pub struct UserApiService {
    node_service: Arc<ZiskNodeService>,
}

impl UserApiService {
    pub fn new(node_service: Arc<ZiskNodeService>) -> Self {
        Self { node_service }
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Relays a coordinator `JobStateEvent` stream as a user `JobEvent` stream.
///
/// Deduplicates consecutive `Running` events with the same phase, and closes
/// the stream after the first terminal event (Completed / Failed / Cancelled).
fn relay_coord_stream(
    mut coord_stream: tonic::Streaming<coord::JobStateEvent>,
) -> BoxStream<JobEvent> {
    Box::pin(stream! {
        let mut last_phase: Option<String> = None;

        while let Some(item) = coord_stream.next().await {
            match item {
                Err(e) => { yield Err(e); return; }
                Ok(state_event) => {
                    // Deduplicate consecutive Running events with the same phase.
                    if state_event.state == "Running" {
                        if state_event.phase == last_phase { continue; }
                        last_phase = state_event.phase.clone();
                    }

                    match coord_state_to_job_event(state_event) {
                        Err(e) => { yield Err(e); return; }
                        Ok(None) => {}
                        Ok(Some(ev)) => {
                            let terminal = matches!(
                                ev.event,
                                Some(job_event::Event::Completed(_)
                                    | job_event::Event::Failed(_)
                                    | job_event::Event::Cancelled(_))
                            );
                            yield Ok(ev);
                            if terminal { return; }
                        }
                    }
                }
            }
        }

        // Coordinator stream closed without a terminal event.
        yield Err(Status::internal("coordinator watch stream ended without terminal state"));
    })
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

    async fn prove(
        &self,
        request: Request<ProveRequest>,
    ) -> Result<Response<ProveResponse>, Status> {
        let req = request.into_inner();

        if req.program_id.is_empty() {
            return Err(Status::invalid_argument("program_id is required"));
        }

        let input = req.input.ok_or_else(|| Status::invalid_argument("input field is required"))?;

        let proof_input = match input.input {
            Some(input_kind::Input::Inputs(path_or_url)) => ProofInputSource::Path(path_or_url),
            Some(input_kind::Input::Inline(chunk)) => {
                if !chunk.is_last {
                    // TODO: create a UnixSocketStreamWriter, pass socket URI to
                    // coordinator, and relay PushJobInput chunks to that socket.
                    return Err(Status::unimplemented("multi-chunk inline input not yet supported"));
                }
                ProofInputSource::Inline(chunk.data)
            }
            Some(input_kind::Input::Stream(uri)) => ProofInputSource::Stream(uri),
            None => return Err(Status::invalid_argument("input.input field is required")),
        };

        let job_id = self
            .node_service
            .launch_proof(LaunchProofParams {
                program_id: req.program_id,
                compute_capacity: 1, // TODO: wire to NodeConfig
                minimal_compute_capacity: 1,
                input: proof_input,
            })
            .await
            .map_err(Status::from)?;

        Ok(Response::new(ProveResponse { job_id }))
    }

    type WatchJobStream = BoxStream<JobEvent>;

    async fn watch_job(
        &self,
        request: Request<WatchJobRequest>,
    ) -> Result<Response<Self::WatchJobStream>, Status> {
        let job_id = request.into_inner().job_id;
        let coord_stream =
            self.node_service.watch_job_stream(job_id).await.map_err(Status::from)?;
        Ok(Response::new(relay_coord_stream(coord_stream)))
    }

    async fn list_jobs(
        &self,
        _request: Request<ListJobsRequest>,
    ) -> Result<Response<ListJobsResponse>, Status> {
        let jobs = self.node_service.list_jobs().await.map_err(Status::from)?;
        Ok(Response::new(ListJobsResponse { jobs: jobs.into_iter().map(Into::into).collect() }))
    }

    async fn get_job(&self, request: Request<GetJobRequest>) -> Result<Response<JobInfo>, Status> {
        let job_id = request.into_inner().job_id;
        let info = self.node_service.get_job(job_id).await.map_err(Status::from)?;
        Ok(Response::new(info.into()))
    }

    async fn wait_job_result(
        &self,
        request: Request<WaitJobResultRequest>,
    ) -> Result<Response<JobInfo>, Status> {
        const MIN_SECS: u32 = 1;
        const DEFAULT_SECS: u32 = 5;

        let req = request.into_inner();
        let timeout_seconds = req.timeout_seconds.unwrap_or(DEFAULT_SECS).max(MIN_SECS);

        let info =
            self.node_service.wait_job(req.job_id, timeout_seconds).await.map_err(Status::from)?;
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
