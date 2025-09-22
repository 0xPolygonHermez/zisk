use anyhow::Result;
use tonic::transport::Channel;
use tracing::{error, info};
use zisk_distributed_grpc_api::{
    zisk_distributed_api_client::ZiskDistributedApiClient, LaunchProofRequest,
};

/// Handle the prove-block subcommand - makes RPC request to coordinator
pub async fn handle(
    coordinator_url: String,
    input_path: String,
    compute_capacity: u32,
    simulated_node: Option<u32>,
) -> Result<()> {
    // Connect to the coordinator
    info!("Connecting to ZisK Coordinator gRPC service on {}", coordinator_url);

    let channel = Channel::from_shared(coordinator_url)?.connect().await?;
    let mut client = ZiskDistributedApiClient::new(channel);

    let launch_proof_request = LaunchProofRequest {
        block_id: "0x1234567890abcdef".into(), // TODO! Placeholder block ID
        compute_capacity,
        input_path,
        simulated_node,
    };

    // Make the RPC call
    info!(
        "Sending Launch request for block id: {} with {} compute units",
        launch_proof_request.block_id, launch_proof_request.compute_capacity
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
