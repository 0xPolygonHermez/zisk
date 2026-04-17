use std::pin::Pin;

use tokio_stream::Stream;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};

use crate::proto::{
    health_check_response::ServingStatus, health_server::Health, HealthCheckRequest,
    HealthCheckResponse,
};

/// Services that can be individually queried by name.
const KNOWN_SERVICES: &[&str] = &["", "zisk.gateway.v1.ZiskGatewayApi", "zisk.gateway.v1.Health"];

pub struct HealthService {
    cancel: CancellationToken,
}

impl HealthService {
    pub fn new(cancel: CancellationToken) -> Self {
        Self { cancel }
    }

    fn current_status(&self) -> ServingStatus {
        if self.cancel.is_cancelled() {
            ServingStatus::NotServing
        } else {
            ServingStatus::Serving
        }
    }

    fn check_known(service: &str) -> Result<(), Status> {
        if KNOWN_SERVICES.contains(&service) {
            Ok(())
        } else {
            Err(Status::not_found(format!("unknown service: {service}")))
        }
    }
}

type WatchStream =
    Pin<Box<dyn Stream<Item = Result<HealthCheckResponse, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl Health for HealthService {
    /// One-shot probe. Used by k8s liveness/readiness and grpc-health-probe.
    async fn check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Self::check_known(&request.into_inner().service)?;
        Ok(Response::new(HealthCheckResponse { status: self.current_status().into() }))
    }

    type WatchStream = WatchStream;

    /// Streaming probe. Emits SERVING immediately, then NOT_SERVING when the
    /// cancellation token fires (graceful shutdown). Stream closes after NOT_SERVING.
    async fn watch(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        Self::check_known(&request.into_inner().service)?;

        let cancel = self.cancel.clone();
        let initial = self.current_status();

        let stream = async_stream::stream! {
            yield Ok(HealthCheckResponse { status: initial.into() });

            if initial == ServingStatus::NotServing {
                return; // already draining — close immediately
            }

            cancel.cancelled().await;
            yield Ok(HealthCheckResponse { status: ServingStatus::NotServing.into() });
        };

        Ok(Response::new(Box::pin(stream)))
    }
}
