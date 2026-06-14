//! Remote backend client — connects to a ZisK Coordinator for distributed proving.

pub(crate) mod execute;
pub(crate) mod prove;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod wrap;

use crate::{Result, SdkError};
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
    remote::setup::SetupByIdRequest,
    setup::{SetupRequest, SetupResult},
    upload::{UploadRequest, UploadResult},
    wrap::WrapRequest,
    Client, ExecutorKind,
};

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

/// Builder for a remote client.
///
/// The `Out` type parameter selects what [`build`](Self::build) returns, and is fixed by the
/// constructor used:
/// - [`ProverClient::remote`](crate::ProverClient::remote) → `Out = RemoteClient` (the concrete,
///   fully-typed client).
/// - [`ZiskClient::remote`](crate::ZiskClient::remote) → `Out = ZiskClient` (the runtime-dispatch
///   façade).
///
/// The parameter is inferred at call sites and never needs to be named.
pub struct RemoteClientBuilder<Out = RemoteClient> {
    url: String,
    connect_timeout: Duration,
    request_timeout: Duration,
    _out: std::marker::PhantomData<fn() -> Out>,
}

impl<Out> RemoteClientBuilder<Out> {
    pub(crate) fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            _out: std::marker::PhantomData,
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
}

impl<Out: From<RemoteClient>> RemoteClientBuilder<Out> {
    /// Build the client.
    ///
    /// Returns the type fixed by the constructor: a [`RemoteClient`] via
    /// [`ProverClient::remote`](crate::ProverClient::remote), or an
    /// [`ZiskClient`](crate::ZiskClient) via [`ZiskClient::remote`](crate::ZiskClient::remote).
    pub fn build(self) -> Result<Out> {
        crate::client::ensure_single_instance();
        let gw = CoordinatorClient::connect(self.url, self.connect_timeout, self.request_timeout)
            .map_err(SdkError::backend)?;
        Ok(RemoteClient { gw }.into())
    }
}

/// Remote client implementation.
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

    /// Submit a ROM setup request
    #[must_use]
    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    /// Submit a ROM setup request for an already-uploaded program by `hash_id`.
    ///
    /// Skips upload — the coordinator must already hold the program's ELF (e.g. from
    /// a prior [`upload`](Self::upload)). Returns `ProgramNotFound` from the coordinator
    /// otherwise.
    #[must_use]
    pub fn setup_by_id(&self, hash_id: impl Into<String>) -> SetupByIdRequest<'_> {
        SetupByIdRequest::new(self, hash_id.into())
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
        InputSource::Stdin(s) => Ok((
            DomainInputKind::try_inline(s.into_inner().read_data()).map_err(SdkError::backend)?,
            None,
        )),
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
            source.open().map_err(SdkError::backend)?;
            let mut data = Vec::new();
            while let Some(chunk) = source.next().map_err(SdkError::backend)? {
                data.extend(chunk);
            }
            source.close().map_err(SdkError::backend)?;
            Ok((Some(DomainInputKind::try_inline(data).map_err(SdkError::backend)?), None))
        }
    }
}
