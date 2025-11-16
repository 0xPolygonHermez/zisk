use std::path::PathBuf;

use anyhow::Result;
use tonic::transport::Channel;
use tracing::{error, info};
use zisk_distributed_coordinator::Config;
use zisk_distributed_grpc_api::{
    zisk_distributed_api_client::ZiskDistributedApiClient, InputMode, LaunchProofRequest,
};

/// Handle the prove subcommand - makes RPC request to coordinator
pub async fn handle(
    coordinator_url: Option<String>,
    data_id: Option<String>,
    input_path: Option<PathBuf>,
    direct_inputs: bool,
    compute_capacity: u32,
    simulated_node: Option<u32>,
    minimal_compute_capacity: Option<u32>,
) -> Result<()> {
    // Initialize tracing - keep guard alive for application lifetime
    let _log_guard = zisk_distributed_common::tracing::init(None, None)?;

    let coordinator_url = coordinator_url.unwrap_or_else(Config::default_url);

    // Connect to the coordinator
    info!("Connecting to ZisK Coordinator gRPC service on {}", coordinator_url);

    let channel = Channel::from_shared(coordinator_url)?.connect().await?;
    let mut client = ZiskDistributedApiClient::new(channel);

    let (input_mode, input_path) = if let Some(ref path) = input_path {
        if path.as_os_str().is_empty() {
            return Err(anyhow::anyhow!("Input path cannot be empty"));
        }

        let input_path = Some(path.to_string_lossy().to_string());

        let input_mode = if direct_inputs { InputMode::Data } else { InputMode::Path };

        (input_mode, input_path)
    } else {
        (InputMode::None, None)
    };

    // ID will be id if present, else input file name or random UUID
    let data_id = if let Some(id) = data_id {
        id
    } else if let Some(ref path) = input_path {
        PathBuf::from(path).file_stem().unwrap().to_string_lossy().to_string()
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    let launch_proof_request = LaunchProofRequest {
        data_id,
        compute_capacity,
        minimal_compute_capacity,
        input_mode: input_mode.into(),
        input_path,
        simulated_node,
    };

    // Make the RPC call
    info!(
        "Sending Launch request for data id: {} with {} compute units",
        launch_proof_request.data_id, launch_proof_request.compute_capacity
    );
    let response = client.launch_proof(launch_proof_request).await?;

    match response.into_inner().result {
        Some(zisk_distributed_grpc_api::launch_proof_response::Result::JobId(job_id)) => {
            info!("Proof job started successfully with job_id: {}", job_id);
        }
        Some(zisk_distributed_grpc_api::launch_proof_response::Result::Error(error)) => {
            error!("Proof job failed: {} - {}", error.code, error.message);
        }
        None => {
            info!("Received empty response from coordinator");
        }
    }

    Ok(())
}
