//! Remote backend client for distributed proof generation.
//!
//! Connects to a remote coordinator to offload proving work to a distributed network.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::Duration;
use tonic::transport::Channel;
use zisk_common::{ProofMode, ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};
use zisk_distributed_grpc_api::{
    zisk_distributed_api_client::ZiskDistributedApiClient, HealthCheckRequest, HintsMode,
    InputMode, JobStatusRequest, LaunchProofRequest,
};
use zisk_prover_backend::{GuestProgram, ProofOpts};

use crate::cancel::CancellationToken;
use crate::execute::ExecuteResult;
use crate::input::ProgramInput;
use crate::proof::Proof;
use crate::{Client, ExecutorKind};

/// Configuration for the remote prover backend.
#[derive(Clone)]
pub struct RemoteClientConfig {
    /// Coordinator URL (e.g., "http://localhost:50051").
    pub(crate) url: String,
    /// Connection timeout.
    pub(crate) connect_timeout: Duration,
    /// Request timeout for individual operations.
    pub(crate) request_timeout: Duration,
}

impl Default for RemoteClientConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:50051".to_string(),
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(300),
        }
    }
}

/// Builder for a remote [`ProverClient`].
pub(crate) struct RemoteClientBuilder {
    config: RemoteClientConfig,
}

impl RemoteClientBuilder {
    pub(crate) fn new(config: RemoteClientConfig) -> Self {
        Self { config }
    }

    pub(crate) async fn build(self) -> Result<RemoteClient> {
        let endpoint = tonic::transport::Endpoint::from_shared(self.config.url.clone())
            .context("Invalid coordinator URL")?
            .connect_timeout(self.config.connect_timeout)
            .timeout(self.config.request_timeout);

        let channel = endpoint.connect().await.context("Failed to connect to coordinator")?;

        // Verify connectivity with health check
        let mut client = ZiskDistributedApiClient::new(channel.clone());
        client
            .health_check(HealthCheckRequest {})
            .await
            .context("Coordinator health check failed")?;

        Ok(RemoteClient { channel, config: self.config })
    }

    /// Build synchronously (blocks on the async connection).
    pub(crate) fn build_sync(self) -> Result<RemoteClient> {
        let config = self.config;
        tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(RemoteClientBuilder::new(config.clone()).build()))
            .unwrap_or_else(|_| {
                // No runtime available, create a temporary one
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime")
                    .block_on(RemoteClientBuilder::new(config).build())
            })
    }
}

/// Remote client that delegates proving to a distributed coordinator.
pub(crate) struct RemoteClient {
    channel: Channel,
    #[allow(dead_code)]
    config: RemoteClientConfig,
}

impl RemoteClient {
    fn client(&self) -> ZiskDistributedApiClient<Channel> {
        ZiskDistributedApiClient::new(self.channel.clone())
    }

    pub(crate) fn vk(&self, _program: &GuestProgram) -> Result<ZiskProgramVK> {
        // TODO: Remote VK retrieval - may need to be computed locally or cached on coordinator
        anyhow::bail!("Remote VK retrieval not yet implemented")
    }
}

impl Client for RemoteClient {
    fn run_upload(&self, program: &GuestProgram) -> Result<()> {
        // TODO: Upload program ELF to coordinator for caching/setup
        let _elf = program.elf();
        tracing::info!("Remote upload: program upload not yet implemented");
        Ok(())
    }

    fn run_setup(&self, _program: &GuestProgram, _with_hints: bool) -> Result<()> {
        // Setup is handled by the coordinator/workers when proving
        tracing::info!("Remote setup: delegated to coordinator");
        Ok(())
    }

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        _executor: ExecutorKind,
        _mode: ProofMode,
        _opts: ProofOpts,
        cancel: Option<&CancellationToken>,
    ) -> Result<Proof> {
        // Check for cancellation before starting
        if cancel.map_or(false, |t| t.is_cancelled()) {
            anyhow::bail!("Operation was cancelled");
        }

        // For now, we use a blocking approach within the sync interface
        let rt = tokio::runtime::Handle::try_current().map_err(|_| {
            anyhow::anyhow!("Remote proving requires a Tokio runtime. Use prove_async() instead.")
        })?;

        rt.block_on(async {
            let mut client = self.client();

            // Compute data_id from program (first 8 chars of hash_id)
            let data_id = program.program_id.hash_id.chars().take(16).collect::<String>();

            // Determine input/hints mode
            let (inputs_mode, inputs_uri) = match &input {
                ProgramInput::Stdin(_) => (InputMode::Data as i32, None),
                ProgramInput::Hints(_) => (InputMode::None as i32, None),
            };

            let hints_mode = match &input {
                ProgramInput::Hints(_) => HintsMode::Stream as i32,
                ProgramInput::Stdin(_) => HintsMode::None as i32,
            };

            // Launch the proof job
            let response = client
                .launch_proof(LaunchProofRequest {
                    data_id: data_id.clone(),
                    compute_capacity: 1,
                    minimal_compute_capacity: 1,
                    inputs_mode,
                    inputs_uri,
                    hints_mode,
                    hints_uri: None,
                    simulated_node: None,
                    metadata: HashMap::new(),
                    execution_only: false,
                })
                .await
                .context("Failed to launch proof job")?;

            let job_id = match response.into_inner().result {
                Some(zisk_distributed_grpc_api::launch_proof_response::Result::JobId(id)) => id,
                Some(zisk_distributed_grpc_api::launch_proof_response::Result::Error(e)) => {
                    anyhow::bail!("Coordinator error: {} - {}", e.code, e.message)
                }
                None => anyhow::bail!("Empty response from coordinator"),
            };

            tracing::info!("Proof job launched: {}", job_id);

            // Poll for completion
            // TODO: Use streaming API for real-time progress updates
            loop {
                if cancel.map_or(false, |t| t.is_cancelled()) {
                    // TODO: Send cancellation to coordinator
                    anyhow::bail!("Operation was cancelled");
                }

                let status_response = client
                    .job_status(JobStatusRequest { job_id: job_id.clone() })
                    .await
                    .context("Failed to get job status")?;

                let status = match status_response.into_inner().result {
                    Some(zisk_distributed_grpc_api::job_status_response::Result::Job(s)) => s,
                    Some(zisk_distributed_grpc_api::job_status_response::Result::Error(e)) => {
                        anyhow::bail!("Job status error: {} - {}", e.code, e.message)
                    }
                    None => anyhow::bail!("Empty job status response"),
                };

                match status.state.as_str() {
                    "completed" => {
                        // TODO: Fetch the actual proof from coordinator
                        anyhow::bail!(
                            "Proof completed but proof retrieval not yet implemented. Job ID: {}",
                            job_id
                        )
                    }
                    "failed" => {
                        anyhow::bail!("Proof generation failed on coordinator. Job ID: {}", job_id)
                    }
                    "cancelled" => {
                        anyhow::bail!("Proof was cancelled. Job ID: {}", job_id)
                    }
                    _ => {
                        // Still in progress
                        tracing::debug!(
                            "Job {} state: {}, phase: {}",
                            job_id,
                            status.state,
                            status.phase
                        );
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        })
    }

    fn run_execute(
        &self,
        _program: &GuestProgram,
        _input: ProgramInput,
        _executor: ExecutorKind,
        cancel: Option<&CancellationToken>,
    ) -> Result<ExecuteResult> {
        if cancel.map_or(false, |t| t.is_cancelled()) {
            anyhow::bail!("Operation was cancelled");
        }
        // TODO: Remote execution - may be implemented for cost estimation
        anyhow::bail!(
            "Remote execution not yet implemented. Use embedded client for dry-run execution."
        )
    }

    fn run_reduce(
        &self,
        _proof_with_publics: &ZiskProofWithPublicValues,
        _override_publics: Option<&ZiskPublics>,
        _override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        // TODO: Remote reduction
        anyhow::bail!("Remote proof reduction not yet implemented")
    }

    fn run_plonk(
        &self,
        _proof_with_publics: &ZiskProofWithPublicValues,
        _override_publics: Option<&ZiskPublics>,
        _override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        // TODO: Remote PLONK generation
        anyhow::bail!("Remote PLONK generation not yet implemented")
    }
}
