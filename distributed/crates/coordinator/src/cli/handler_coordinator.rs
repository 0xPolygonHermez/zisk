use anyhow::Result;
use std::net::TcpListener;
use tonic::transport::Server;
use tracing::{error, info};
use zisk_distributed_coordinator::{create_shutdown_signal, Config, CoordinatorGrpc};
use zisk_distributed_grpc_api::zisk_distributed_api_server::ZiskDistributedApiServer;

pub async fn handle(port_override: Option<u16>, webhook_url: Option<String>) -> Result<()> {
    // Load configuration
    let config = Config::load(port_override, webhook_url)?;

    // Create coordinator service
    let coordinator_service = CoordinatorGrpc::new(config.clone()).await?;

    // Use command line port if provided, otherwise use config port
    let grpc_port = port_override.unwrap_or(config.server.port);

    let addr = format!("{}:{}", config.server.host, grpc_port);
    let grpc_addr = addr.parse().map_err(|e| {
        error!("Failed to parse address '{}': {}", addr, e);
        anyhow::anyhow!("Invalid address format: {}", e)
    })?;

    // Verify the port is available before starting the coordinator grpc server
    if TcpListener::bind(&addr).is_err() {
        error!(
            "Port {} is already in use on {}. Coordinator gRPC server cannot start.",
            grpc_port, config.server.host
        );
        error!("Please ensure no other service is using this port or configure a different port.");
        return Err(anyhow::anyhow!("Port {} is already in use", grpc_port));
    }

    // Create shutdown signal handler
    let shutdown_signal = create_shutdown_signal();

    // Start the gRPC server with graceful shutdown
    info!("Starting Coordinator Network gRPC service on {addr}");

    // Run the gRPC server with shutdown signal
    tokio::select! {
        result = Server::builder()
            .add_service(ZiskDistributedApiServer::new(coordinator_service))
            .serve(grpc_addr) => {
            match result {
                Ok(_) => {
                    info!("gRPC Server shutdown gracefully");
                }
                Err(e) => {
                    error!("gRPC Server error on {}: {}", addr, e);
                    return Err(e.into());
                }
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, stopping gRPC server");
        }
    }

    Ok(())
}
