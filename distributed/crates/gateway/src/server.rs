//! Gateway gRPC server — builds the tonic server and wires all components.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tonic::transport::Server;
use tracing::info;

use crate::backend::BackendService;
use crate::config::Config as GatewayConfig;
use crate::metrics;
use crate::proto::zisk_gateway_api_server::ZiskGatewayApiServer;
use crate::service::GatewayService;
use crate::shutdown::shutdown_signal;

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

        // Large ELF files can exceed the 4 MB tonic default — configure on the service.
        let svc = ZiskGatewayApiServer::new(service)
            .max_decoding_message_size(64 * 1024 * 1024);

        let serve = Server::builder()
            // Keep WatchJob streams alive through NAT/firewall idle timeouts.
            .http2_keepalive_interval(Some(Duration::from_secs(30)))
            .http2_keepalive_timeout(Some(Duration::from_secs(10)))
            .add_service(svc)
            .serve_with_shutdown(addr, async move {
                shutdown_signal().await;
                info!("shutdown signal received — draining in-flight requests");
            });

        // Hard drain timeout: if in-flight RPCs don't finish within shutdown_secs,
        // we exit anyway. With no active RPCs this returns immediately.
        tokio::time::timeout(Duration::from_secs(shutdown_secs), serve)
            .await
            .unwrap_or(Ok(()))?;

        info!("zisk-gateway stopped");
        Ok(())
    }
}
