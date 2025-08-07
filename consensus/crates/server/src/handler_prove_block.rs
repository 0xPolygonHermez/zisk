use anyhow::Result;
use consensus_api::{consensus_api_client::ConsensusApiClient, StartProofRequest};
use tonic::transport::Channel;
use tracing::info;

/// Handle the prove-block subcommand - makes RPC request to coordinator
pub async fn handle(server_url: String, input_path: String, num_provers: u32) -> Result<()> {
    // Connect to the gRPC server
    info!("Connecting to Consensus Network gRPC service on {}", server_url);

    let channel = Channel::from_shared(server_url)?.connect().await?;
    let mut client = ConsensusApiClient::new(channel);

    let start_proof_request = StartProofRequest {
        block_id: 0, // Placeholder block ID
        num_provers,
        input_path,
    };

    // Make the RPC call
    info!("Sending StartProof request for {} provers", num_provers);
    let response = client.start_proof(start_proof_request).await?;

    match response.into_inner().result {
        Some(consensus_api::start_proof_response::Result::JobId(job_id)) => {
            info!("Proof job started successfully with job_id: {}", job_id);
        }
        Some(consensus_api::start_proof_response::Result::Error(error)) => {
            info!("Proof job failed: {} - {}", error.code, error.message);
        }
        None => {
            info!("Received empty response from coordinator");
        }
    }

    Ok(())
}
