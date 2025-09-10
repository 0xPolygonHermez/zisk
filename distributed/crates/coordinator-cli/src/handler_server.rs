use anyhow::Result;
use distributed_config::Config;
use distributed_coordinator::{shutdown::create_shutdown_signal, ConsensusService};
use distributed_grpc_api::distributed_api_server::DistributedApiServer;
use std::net::TcpListener;
use tonic::transport::Server;
use tracing::{error, info};

/// Handle the server mode (default behavior)
pub async fn handle(port_override: Option<u16>) -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Create consensus service
    let consensus_service = ConsensusService::new(config.clone()).await?;

    // Get a reference to the communication manager for shutdown
    let comm_manager = consensus_service.comm_manager.clone();

    // Use command line port if provided, otherwise use config port
    let grpc_port = port_override.unwrap_or(config.server.port);

    let addr = format!("{}:{}", config.server.host, grpc_port);
    let grpc_addr = addr.parse().map_err(|e| {
        error!("Failed to parse address '{}': {}", addr, e);
        anyhow::anyhow!("Invalid address format: {}", e)
    })?;

    // Verify the port is available before starting the server
    if TcpListener::bind(&addr).is_err() {
        error!(
            "Port {} is already in use on {}. Server cannot start.",
            grpc_port, config.server.host
        );
        error!("Please ensure no other service is using this port or configure a different port.");
        return Err(anyhow::anyhow!("Port {} is already in use", grpc_port));
    }

    // Create shutdown signal handler
    let shutdown_signal = create_shutdown_signal();

    // Start the gRPC server with graceful shutdown
    info!("Starting Consensus Network gRPC service on {addr}");

    // Run the gRPC server with shutdown signal
    tokio::select! {
        result = Server::builder()
            .add_service(DistributedApiServer::new(consensus_service))
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

    // Cleanup service state
    info!("Shutting down Consensus Network gRPC service on {addr}");
    if let Err(e) = comm_manager.shutdown().await {
        error!("Error during shutdown: {}", e);
    }

    Ok(())
}
