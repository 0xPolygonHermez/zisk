//! Remote backend client — connects to a ZisK Gateway for distributed proving.

pub(crate) mod execute;
pub(crate) mod prove;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod wrap;

use anyhow::{Context, Result};
use std::time::Duration;
use tonic::transport::Channel;
use zisk_common::{ZiskProgramVK, ZiskProofWithPublicValues};
use zisk_gateway_grpc_api::{
    proto::{InputChunk, InputKind, JobKind, JobRequestMessage, RegisterGuestProgramRequest},
    ZiskGatewayApiClient,
};
use zisk_prover_backend::GuestProgram;

use crate::{input::ProgramInput, ProverOpts};

/// Configuration for the remote prover backend.
#[derive(Clone)]
pub struct RemoteClientConfig {
    /// Gateway URL (e.g., "http://localhost:50051").
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

pub(crate) struct RemoteClientBuilder {
    config: RemoteClientConfig,
    #[allow(dead_code)]
    prover_options: ProverOpts,
}

impl RemoteClientBuilder {
    pub(crate) fn new(config: RemoteClientConfig) -> Self {
        Self { config, prover_options: ProverOpts::default() }
    }

    #[must_use]
    pub(crate) fn with_prover_options(mut self, opts: ProverOpts) -> Self {
        self.prover_options = opts;
        self
    }

    async fn connect(config: &RemoteClientConfig) -> Result<Channel> {
        tonic::transport::Endpoint::from_shared(config.url.clone())
            .context("Invalid gateway URL")?
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .connect()
            .await
            .context("Failed to connect to gateway")
    }

    pub(crate) fn build_sync(self) -> Result<RemoteClient> {
        let channel = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(Self::connect(&self.config))
        })?;
        Ok(RemoteClient { gateway: ZiskGatewayApiClient::new(channel) })
    }
}

pub(crate) struct RemoteClient {
    pub(crate) gateway: ZiskGatewayApiClient<Channel>,
}

impl RemoteClient {
    pub(crate) async fn register_program(&self, elf: Vec<u8>) -> Result<String> {
        let mut gw = self.gateway.clone();
        let resp = gw
            .register_guest_program(RegisterGuestProgramRequest { zisk_elf: elf })
            .await
            .context("RegisterGuestProgram RPC failed")?;
        Ok(resp.into_inner().hash_id)
    }

    pub(crate) async fn submit_job(&self, kind: JobKind) -> Result<String> {
        let mut gw = self.gateway.clone();
        let resp = gw
            .job_request(JobRequestMessage { job_kind: Some(kind) })
            .await
            .context("JobRequest RPC failed")?;
        Ok(resp.into_inner().job_id)
    }

    /// Submit a job, blocking the calling thread. Requires a live tokio runtime.
    pub(crate) fn submit_job_sync(&self, kind: JobKind) -> Result<String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.submit_job(kind))
        })
    }

    /// Register a program, blocking the calling thread. Requires a live tokio runtime.
    pub(crate) fn register_program_sync(&self, elf: &[u8]) -> Result<String> {
        let elf = elf.to_vec();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.register_program(elf))
        })
    }

    pub(crate) fn gateway_client(&self) -> ZiskGatewayApiClient<Channel> {
        self.gateway.clone()
    }

    pub(crate) fn vk(&self, _program: &GuestProgram) -> Result<ZiskProgramVK> {
        anyhow::bail!("Remote VK retrieval not yet implemented")
    }
}

// ── Shared helpers used across remote sub-modules ─────────────────────────────

pub(crate) fn stdin_to_input_kind(input: ProgramInput) -> Result<InputKind> {
    match input {
        ProgramInput::Stdin(s) => {
            let data = s.into_inner().read_data();
            Ok(InputKind {
                kind: Some(zisk_gateway_grpc_api::proto::input_kind::Kind::Inline(InputChunk {
                    data,
                    is_last: true,
                })),
            })
        }
        ProgramInput::Hints(_) => {
            anyhow::bail!("Hints input is not supported for remote proving")
        }
    }
}

pub(crate) fn duration_to_proto_timestamp(d: Duration) -> prost_types::Timestamp {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let deadline = now + d;
    prost_types::Timestamp {
        seconds: deadline.as_secs() as i64,
        nanos: deadline.subsec_nanos() as i32,
    }
}

pub(crate) fn proof_with_publics_to_proto(
    proof: &ZiskProofWithPublicValues,
    proof_kind: zisk_gateway_grpc_api::proto::ProofKind,
) -> Result<zisk_gateway_grpc_api::proto::Proof> {
    let data =
        bincode::serialize(proof).map_err(|e| anyhow::anyhow!("failed to serialize proof: {e}"))?;
    Ok(zisk_gateway_grpc_api::proto::Proof {
        proof_id: uuid::Uuid::new_v4().to_string(),
        hash_id: String::new(),
        verification_key: Vec::new(),
        proof_kind: proof_kind as i32,
        data,
        public_inputs: Vec::new(),
        started_at: None,
        completed_at: None,
    })
}
