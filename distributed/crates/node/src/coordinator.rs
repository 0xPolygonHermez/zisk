use tonic::transport::Channel;
use tonic::Response;
use zisk_distributed_grpc_api::zisk_coordinator_api_client::ZiskCoordinatorApiClient;
use zisk_distributed_grpc_api::{
    DeleteProgramRequest, DeleteProgramResponse, GetProgramRequest, GetProgramResponse,
    JobStatusResponse, ListProgramsRequest, ListProgramsResponse, RegisterProgramRequest,
    RegisterProgramResponse, UpdateProgramRequest, UpdateProgramResponse, WaitJobRequest,
    WaitProgramRequest,
};

/// Thin wrapper around the generated gRPC client for the coordinator's
/// external API (`ZiskCoordinatorApi`).
///
/// Uses lazy connection so the node starts even when the coordinator is
/// temporarily unreachable.
#[derive(Clone)]
pub struct CoordinatorClient {
    pub(crate) inner: ZiskCoordinatorApiClient<Channel>,
}

impl CoordinatorClient {
    pub fn connect(url: String) -> Self {
        let channel = Channel::from_shared(url).expect("valid coordinator URL").connect_lazy();
        Self { inner: ZiskCoordinatorApiClient::new(channel) }
    }

    pub async fn register_program(
        &mut self,
        req: RegisterProgramRequest,
    ) -> Result<Response<RegisterProgramResponse>, tonic::Status> {
        self.inner.register_program(req).await
    }

    pub async fn list_programs(
        &mut self,
        req: ListProgramsRequest,
    ) -> Result<Response<ListProgramsResponse>, tonic::Status> {
        self.inner.list_programs(req).await
    }

    pub async fn get_program(
        &mut self,
        req: GetProgramRequest,
    ) -> Result<Response<GetProgramResponse>, tonic::Status> {
        self.inner.get_program(req).await
    }

    pub async fn update_program(
        &mut self,
        req: UpdateProgramRequest,
    ) -> Result<Response<UpdateProgramResponse>, tonic::Status> {
        self.inner.update_program(req).await
    }

    pub async fn delete_program(
        &mut self,
        req: DeleteProgramRequest,
    ) -> Result<Response<DeleteProgramResponse>, tonic::Status> {
        self.inner.delete_program(req).await
    }

    pub async fn wait_program(
        &mut self,
        req: WaitProgramRequest,
    ) -> Result<Response<GetProgramResponse>, tonic::Status> {
        self.inner.wait_program(req).await
    }

    pub async fn wait_job(
        &mut self,
        req: WaitJobRequest,
    ) -> Result<Response<JobStatusResponse>, tonic::Status> {
        self.inner.wait_job(req).await
    }
}
