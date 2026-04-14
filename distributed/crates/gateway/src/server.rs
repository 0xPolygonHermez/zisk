//! Gateway gRPC server — builds the tonic server and wires all components.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tonic::transport::Server;
use tracing::{info, warn};

use crate::backend::BackendService;
use crate::config::Config as GatewayConfig;
use crate::metrics;
use crate::proto::zisk_gateway_api_server::ZiskGatewayApiServer;
use crate::service::GatewayService;
use crate::shutdown::shutdown_signal;

/// Maximum inbound message size. Large ELF files can exceed the 4 MB tonic default.
const MAX_DECODING_MESSAGE_SIZE: usize = 64 * 1024 * 1024; // 64 MB

pub struct GatewayServer<B: BackendService> {
    config: GatewayConfig,
    backend: Arc<B>,
}

impl<B: BackendService> GatewayServer<B> {
    pub fn new(config: GatewayConfig, backend: B) -> Self {
        Self { config, backend: Arc::new(backend) }
    }

    pub async fn run(self) -> Result<()> {
        // Start metrics HTTP server
        metrics::start(&self.config.metrics).await?;

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

        Server::builder()
            // Keep WatchJob streams alive through NAT/firewall idle timeouts.
            .http2_keepalive_interval(Some(Duration::from_secs(30)))
            .http2_keepalive_timeout(Some(Duration::from_secs(10)))
            .add_service(svc)
            .serve_with_shutdown(addr, async move {
                shutdown_signal().await;
                info!("shutdown signal received — draining in-flight requests");
                // Drain timeout starts only AFTER the signal — the server runs indefinitely before
                // that. If in-flight RPCs don't finish within shutdown_secs we force-close.
                tokio::time::sleep(Duration::from_secs(shutdown_secs)).await;
                warn!(
                    timeout_secs = shutdown_secs,
                    "graceful shutdown timed out — forcing close"
                );
            })
            .await
            .map_err(anyhow::Error::from)?;

        info!("zisk-gateway stopped gracefully");

        Ok(())
    }
}
