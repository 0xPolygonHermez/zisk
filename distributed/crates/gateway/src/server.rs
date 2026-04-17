//! Gateway gRPC server — builds the tonic server and wires all components.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::{info, warn};

use crate::backend::BackendService;
use crate::config::Config as GatewayConfig;
use crate::health::HealthService;
use crate::metrics;
use crate::proto::{health_server::HealthServer, zisk_gateway_api_server::ZiskGatewayApiServer};
use crate::service::GatewayService;
use crate::shutdown::shutdown_signal;

/// Maximum inbound message size. Large ELF files can exceed the 4 MB tonic default.
const MAX_DECODING_MESSAGE_SIZE: usize = 64 * 1024 * 1024; // 64 MB

pub struct GatewayServer<B: BackendService> {
    config: GatewayConfig,
    backend: Arc<B>,
    cancel: CancellationToken,
}

impl<B: BackendService> GatewayServer<B> {
    pub fn new(config: GatewayConfig, backend: B, cancel: CancellationToken) -> Self {
        Self { config, backend: Arc::new(backend), cancel }
    }

    pub async fn run(self) -> Result<()> {
        metrics::start(&self.config.metrics, self.cancel.clone()).await?;

        let addr = self.config.grpc_addr().parse()?;
        let service = GatewayService::new(Arc::clone(&self.backend));
        let shutdown_secs = self.config.server.shutdown_timeout_seconds;

        info!(
            version = %self.config.service.version,
            backend = ?self.config.backend.mode,
            "zisk-gateway listening on {addr}"
        );

        let svc =
            ZiskGatewayApiServer::new(service).max_decoding_message_size(MAX_DECODING_MESSAGE_SIZE);
        let health_svc = HealthServer::new(HealthService::new(self.cancel.clone()));

        let cancel = self.cancel.clone();
        let drain_cancel = cancel.clone();

        // `serve_with_shutdown` stops accepting connections when the signal future
        // returns, then blocks until every open HTTP/2 connection closes (i.e. until
        // every streaming RPC — WatchJob — finishes). We return from the signal future
        // immediately so the drain starts right away, and race it against a hard timeout
        // from outside so long-lived streams don't stall the process indefinitely.
        let server_fut = Server::builder()
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

        info!("zisk-gateway stopped gracefully");

        Ok(())
    }
}
