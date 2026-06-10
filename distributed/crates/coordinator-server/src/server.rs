//! Coordinator gRPC server — builds the tonic server and wires all components.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::{info, warn};

use crate::backend::{BackendService, LiveStateProvider};
use crate::config::Config as CoordinatorServerConfig;
use crate::grpc::GrpcAdapter;
use crate::handler::CoordinatorHandler;
use crate::metrics;
use crate::proto::zisk_coordinator_api_server::ZiskCoordinatorApiServer;
use crate::shutdown::shutdown_signal;
use crate::{HTTP2_CONNECTION_WINDOW_SIZE, HTTP2_STREAM_WINDOW_SIZE};
use zisk_coordinator::JobHistoryStore;

/// Maximum inbound/outbound message size. Must be at least as large as
/// `DomainInputKind::MAX_INLINE_BYTES` in coordinator-api.
const MAX_MESSAGE_SIZE: usize = 128 * 1024 * 1024; // 128 MB

pub struct CoordinatorServer<B: BackendService + LiveStateProvider> {
    config: CoordinatorServerConfig,
    backend: Arc<B>,
    history: Option<Arc<dyn JobHistoryStore>>,
    cancel: CancellationToken,
}

impl<B: BackendService + LiveStateProvider> CoordinatorServer<B> {
    pub fn new(config: CoordinatorServerConfig, backend: B, cancel: CancellationToken) -> Self {
        Self { config, backend: Arc::new(backend), history: None, cancel }
    }

    pub fn new_with_history(
        config: CoordinatorServerConfig,
        backend: B,
        history: Arc<dyn JobHistoryStore>,
        cancel: CancellationToken,
    ) -> Self {
        Self { config, backend: Arc::new(backend), history: Some(history), cancel }
    }

    pub async fn run(self) -> Result<()> {
        metrics::start(
            &self.config.metrics,
            self.cancel.clone(),
            self.history.clone(),
            Some(Arc::clone(&self.backend) as Arc<dyn LiveStateProvider>),
        )
        .await?;

        let addr = self.config.grpc_addr().parse()?;
        let service = GrpcAdapter::new(CoordinatorHandler::new(Arc::clone(&self.backend)));
        let shutdown_secs = self.config.server.shutdown_timeout_seconds;

        info!(
            version = %self.config.service.version,
            backend = ?self.config.backend.mode,
            "zisk-coordinator listening on {addr}"
        );

        let svc = ZiskCoordinatorApiServer::new(service)
            .max_decoding_message_size(MAX_MESSAGE_SIZE)
            .max_encoding_message_size(MAX_MESSAGE_SIZE);

        // Standard grpc.health.v1.Health service — used by grpc_health_probe / k8s.
        let (health_reporter, health_svc) = tonic_health::server::health_reporter();
        health_reporter.set_service_status("", tonic_health::ServingStatus::Serving).await;

        let cancel = self.cancel.clone();
        let drain_cancel = cancel.clone();

        // `serve_with_shutdown` stops accepting connections when the signal future
        // returns, then blocks until every open HTTP/2 connection closes (i.e. until
        // every streaming RPC — WatchJob — finishes). We return from the signal future
        // immediately so the drain starts right away, and race it against a hard timeout
        // from outside so long-lived streams don't stall the process indefinitely.
        let server_fut = Server::builder()
            .initial_connection_window_size(Some(HTTP2_CONNECTION_WINDOW_SIZE))
            .initial_stream_window_size(Some(HTTP2_STREAM_WINDOW_SIZE))
            // Keep WatchJob streams alive through NAT/firewall idle timeouts.
            .http2_keepalive_interval(Some(Duration::from_secs(30)))
            .http2_keepalive_timeout(Some(Duration::from_secs(10)))
            .add_service(health_svc)
            .add_service(svc)
            .serve_with_shutdown(addr, async move {
                shutdown_signal().await;
                info!("shutdown signal received — draining in-flight requests");
                cancel.cancel();
                // Return immediately — tonic now sends graceful_shutdown() to all
                // connections and waits for them to close.
            });

        tokio::select! {
            result = server_fut => result.map_err(anyhow::Error::from)?,
            _ = async move {
                drain_cancel.cancelled().await;
                tokio::time::sleep(Duration::from_secs(shutdown_secs)).await;
                warn!(timeout_secs = shutdown_secs, "graceful drain timeout — forcing close");
            } => {}
        }

        info!("zisk-coordinator stopped gracefully");

        Ok(())
    }
}
