use anyhow::Result;
use distributed_grpc_api::{distributed_api_client::DistributedApiClient, StartProofRequest};
use tonic::transport::Channel;
use tracing::{error, info};

/// Handle the prove-block subcommand - makes RPC request to coordinator
pub async fn handle(server_url: String, input_path: String, compute_capacity: u32) -> Result<()> {
    // Connect to the gRPC server
    info!("Connecting to Consensus Network gRPC service on {}", server_url);

    let channel = Channel::from_shared(server_url)?.connect().await?;
    let mut client = DistributedApiClient::new(channel);

    let start_proof_request = StartProofRequest {
        block_id: "0x1234567890abcdef".into(), // Placeholder block ID
        compute_units: compute_capacity,
        input_path,
    };

    // Make the RPC call
    info!(
        "Sending StartProof request for block id: {} with {} compute units",
        start_proof_request.block_id, start_proof_request.compute_units
    );
    let response = client.start_proof(start_proof_request).await?;

    match response.into_inner().result {
        Some(distributed_grpc_api::start_proof_response::Result::JobId(job_id)) => {
            info!("Proof job started successfully with job_id: {}", job_id);
        }
        Some(distributed_grpc_api::start_proof_response::Result::Error(error)) => {
            error!("Proof job failed: {} - {}", error.code, error.message);
        }
        None => {
            info!("Received empty response from coordinator");
        }
    }

    Ok(())
}
