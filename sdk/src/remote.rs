//! Remote backend client — connects to a ZisK Coordinator for distributed proving.

pub(crate) mod execute;
pub(crate) mod prove;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod wrap;

use anyhow::Result;
use std::time::Duration;
use zisk_common::io::StreamRead;
use zisk_common::{ProgramVK, Proof, ProofKind, PublicValues};
use zisk_coordinator_api::dto::DomainInputKind;
use zisk_coordinator_client::CoordinatorClient;
use zisk_prover_backend::GuestProgram;

use crate::{
    execute::{ExecuteRequest, ExecuteResult},
    hints::HintsSource,
    input_source::InputSource,
    job_handle::{JobHandle, SubscriberList},
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
        let gw = CoordinatorClient::connect(self.url, self.connect_timeout, self.request_timeout)?;
        Ok(RemoteClient { gw })
    }
}

#[derive(Clone)]
pub struct RemoteClient {
    pub(crate) gw: CoordinatorClient,
}

impl Client for RemoteClient {
    fn run_upload(&self, program: &GuestProgram) -> Result<UploadResult> {
        self.do_upload(program)
    }

    fn run_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        self.do_setup(program, with_hints, emulator_only, timeout, subs)
    }

    fn run_prove(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<crate::prove::ProveResult>> {
        self.do_prove(program, stdin, hints, executor, proof_kind, timeout, subs)
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteResult>> {
        self.do_execute(program, stdin, hints, executor, timeout, subs)
    }

    fn run_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        _override_publics: Option<PublicValues>,
        _override_program_vk: Option<ProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<crate::prove::ProveResult>> {
        self.do_wrap(proof, proof_kind, timeout, subs)
    }
}

impl RemoteClient {
    /// Submit a prove request.
    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, stdin, ExecutorKind::default())
    }

    /// Submit an execute request (dry-run, no proof).
    #[must_use]
    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> ExecuteRequest<'a, Self> {
        ExecuteRequest::new(self, program, stdin, ExecutorKind::default())
    }

    /// Submit a ROM setup request.
    #[must_use]
    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    /// Upload/register the program ELF with the coordinator.
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

/// Converts an [`InputSource`] into a `DomainInputKind` for submission.
///
/// For stream-backed input, also returns the [`ZiskStream`](crate::ZiskStream)
/// so the caller can spawn an activate thread or inject a gRPC sender.
pub(crate) fn stdin_to_input_kind(
    stdin: InputSource,
) -> Result<(DomainInputKind, Option<crate::input_stream::ZiskStream>)> {
    match stdin {
        InputSource::Stream(stream) => {
            // All stream types (unix, quic, grpc) send StreamUri so workers
            // start in streaming mode (InputNull).  For grpc://, the coordinator
            // skips the relay — data arrives via PushJobInput instead.
            Ok((DomainInputKind::StreamUri(stream.uri().to_string()), Some(stream)))
        }
        InputSource::Stdin(s) => {
            Ok((DomainInputKind::try_inline(s.into_inner().read_data())?, None))
        }
    }
}

/// Converts an optional [`HintsSource`] into a `DomainInputKind` for submission.
///
/// For stream-backed hints, also returns the [`ZiskStream`](crate::ZiskStream)
/// so the caller can inject a gRPC sender after job submission.
///
/// - `HintsSource::Hints` → reads data and sends inline
/// - `HintsSource::Stream` → `StreamUri(stream.uri())`
pub(crate) fn hints_to_input_kind(
    hints: Option<HintsSource>,
) -> Result<(Option<DomainInputKind>, Option<crate::input_stream::ZiskStream>)> {
    let hints = match hints {
        Some(h) => h,
        None => return Ok((None, None)),
    };
    match hints {
        HintsSource::Stream(stream) => {
            Ok((Some(DomainInputKind::StreamUri(stream.uri().to_string())), Some(*stream)))
        }
        HintsSource::Hints(h) => {
            let mut source = h.into_inner();
            source.open()?;
            let mut data = Vec::new();
            while let Some(chunk) = source.next()? {
                data.extend(chunk);
            }
            source.close()?;
            Ok((Some(DomainInputKind::try_inline(data)?), None))
        }
    }
}
