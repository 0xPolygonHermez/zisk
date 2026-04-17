//! Remote backend client — connects to a ZisK Gateway for distributed proving.

pub(crate) mod execute;
pub(crate) mod prove;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod wrap;

use anyhow::{Context, Result};
use std::time::Duration;
use tonic::transport::Channel;
use zisk_common::{ProgramVK, Proof, ProofKind, PublicValues};
use zisk_gateway::backend::{DomainInputChunk, DomainInputKind, DomainJobKind, DomainProof};
use zisk_gateway_api::{proto::JobRequestMessage, ZiskGatewayApiClient};
use zisk_prover_backend::{GuestProgram, ProveOutput};

use crate::{
    execute::ExecuteRequest,
    input::ProgramInput,
    job_handle::{JobHandle, JobId, SubscriberList},
    prove::ProveRequest,
    setup::{SetupRequest, SetupResult},
    upload::{UploadRequest, UploadResult},
    wrap::WrapRequest,
    Client, ExecutorKind,
};

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub struct RemoteClientBuilder {
    url: String,
    connect_timeout: Duration,
    request_timeout: Duration,
}

impl RemoteClientBuilder {
    pub(crate) fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }

    /// Override the connection timeout. Default: 10 s.
    #[must_use]
    pub fn connect_timeout(mut self, d: Duration) -> Self {
        self.connect_timeout = d;
        self
    }

    /// Override the per-request timeout. Default: 300 s.
    #[must_use]
    pub fn request_timeout(mut self, d: Duration) -> Self {
        self.request_timeout = d;
        self
    }

    /// Build the [`RemoteClient`].
    pub fn build(self) -> Result<RemoteClient> {
        crate::client::ensure_single_instance();
        let channel = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                tonic::transport::Endpoint::from_shared(self.url)
                    .context("Invalid gateway URL")?
                    .connect_timeout(self.connect_timeout)
                    .timeout(self.request_timeout)
                    .connect()
                    .await
                    .context("Failed to connect to gateway")
            })
        })?;
        Ok(RemoteClient { gw_client: ZiskGatewayApiClient::new(channel) })
    }
}

#[derive(Clone)]
pub struct RemoteClient {
    pub(crate) gw_client: ZiskGatewayApiClient<Channel>,
}

impl RemoteClient {
    pub(crate) fn submit_job(&self, kind: DomainJobKind) -> Result<JobId> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut gw = self.gw_client.clone();
                let resp = gw
                    .job_request(JobRequestMessage { job_kind: Some(kind.into()) })
                    .await
                    .context("JobRequest RPC failed")?;

                let job_id = resp.into_inner().job_id;

                Ok(JobId::from(job_id))
            })
        })
    }
}

impl Client for RemoteClient {
    fn run_upload(&self, program: &GuestProgram) -> Result<UploadResult> {
        self.do_upload(program)
    }

    fn run_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        self.do_setup(program, with_hints, timeout, subs)
    }

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveOutput>> {
        self.do_prove(program, input, executor, proof_kind, timeout, subs)
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<zisk_prover_backend::ExecuteOutput>> {
        self.do_execute(program, input, executor, timeout, subs)
    }

    fn run_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        _override_publics: Option<PublicValues>,
        _override_program_vk: Option<ProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveOutput>> {
        self.do_wrap(proof, proof_kind, timeout, subs)
    }
}

impl RemoteClient {
    /// Submit a prove request.
    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, input)
    }

    /// Submit an execute request (dry-run, no proof).
    #[must_use]
    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ExecuteRequest<'a, Self> {
        ExecuteRequest::new(self, program, input)
    }

    /// Submit a ROM setup request.
    #[must_use]
    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    /// Upload/register the program ELF with the gateway.
    #[must_use]
    pub fn upload<'a>(&'a self, program: &'a GuestProgram) -> UploadRequest<'a, Self> {
        UploadRequest::new(self, program)
    }

    /// Submit a wrap/convert proof request.
    #[must_use]
    pub fn wrap_proof<'a>(
        &'a self,
        proof: &'a Proof,
        proof_kind: ProofKind,
    ) -> WrapRequest<'a, Self> {
        WrapRequest::new(self, proof, proof_kind)
    }
}

// ── Shared helpers used across remote sub-modules ─────────────────────────────

pub(crate) fn deadline_from_now(d: Duration) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() + chrono::Duration::from_std(d).unwrap()
}

pub(crate) fn stdin_to_input_kind(input: ProgramInput) -> Result<DomainInputKind> {
    match input {
        ProgramInput::Stdin(s) => {
            let data = s.into_inner().read_data();
            Ok(DomainInputKind::Inline(DomainInputChunk { data, is_last: true }))
        }
        ProgramInput::Hints(_) => {
            anyhow::bail!("Hints input is not supported for remote proving")
        }
    }
}

pub(crate) fn proof_to_proto(proof: &Proof, proof_kind: ProofKind) -> Result<DomainProof> {
    let data =
        bincode::serialize(proof).map_err(|e| anyhow::anyhow!("failed to serialize proof: {e}"))?;
    Ok(DomainProof {
        proof_id: uuid::Uuid::new_v4(),
        hash_id: String::new(),
        verification_key: Vec::new(),
        proof_kind: proof_kind.into(),
        data,
        public_inputs: Vec::new(),
        started_at: None,
        completed_at: None,
    })
}
