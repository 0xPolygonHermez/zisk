use anyhow::Result;
use cargo_zisk::ux::print_banner;
use colored::Colorize;
use std::net::TcpListener;
use tonic::transport::Server;
use tracing::{error, info};
use zisk_distributed_coordinator::{create_shutdown_signal, Config, CoordinatorGrpc};
use zisk_distributed_grpc_api::zisk_distributed_api_server::ZiskDistributedApiServer;

pub async fn handle(
    config_file: Option<String>,
    port: Option<u16>,
    webhook_url: Option<String>,
) -> Result<()> {
    // Config file is now optional - if not provided, defaults will be used
    let config_file = config_file.or_else(|| std::env::var("ZISK_COORDINATOR_CONFIG_PATH").ok());

    // Load configuration
    let config = Config::load(config_file, port, webhook_url)?;

    // Initialize tracing - keep guard alive for application lifetime
    let _log_guard = zisk_distributed_common::tracing::init(Some(&config.logging))?;

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let grpc_addr = addr.parse().map_err(|e| {
        error!("Failed to parse address '{}': {}", addr, e);
        anyhow::anyhow!("Invalid address format: {}", e)
    })?;

    print_banner();
    print_command_info(&config, &addr);

    // Verify the port is available before starting the coordinator grpc server
    if TcpListener::bind(&addr).is_err() {
        error!(
            "Port {} is already in use on {}. Coordinator gRPC server cannot start.",
            config.server.port, config.server.host
        );
        error!("Please ensure no other service is using this port or configure a different port.");
        return Err(anyhow::anyhow!("Port {} is already in use", config.server.port));
    }

    // Start the gRPC server with graceful shutdown
    info!("Starting Coordinator Network gRPC service on {addr}");

    // Create coordinator service
    let coordinator_service = CoordinatorGrpc::new(config.clone()).await?;

    // Create shutdown signal handler
    let shutdown_signal = create_shutdown_signal();

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

fn print_command_info(config: &Config, addr: &str) {
    println!(
        "{} zisk-coordinator ({} {})",
        format!("{: >12}", "Command").bright_green().bold(),
        config.service.name,
        config.service.version
    );
    println!("{: >12} {}", "Environment".bright_green().bold(), config.service.environment);
    println!(
        "{: >12} {}/{} {}",
        "Logging".bright_green().bold(),
        config.logging.level,
        config.logging.format,
        format!("(log file: {})", config.logging.file_path.as_deref().unwrap_or_default())
            .bright_black()
    );

    println!("{: >12} {}", "Host/Port".bright_green().bold(), addr);

    println!();
}
