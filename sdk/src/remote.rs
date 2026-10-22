//! Remote backend client — connects to a ZisK Coordinator for distributed proving.

pub(crate) mod execute;
pub(crate) mod prove;
pub(crate) mod recurser;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod wrap;

use crate::{Result, SdkError};
use std::time::Duration;
use zisk_common::io::{BytesPushSender, StreamRead};
use zisk_common::{ProgramVK, Proof, ProofKind, PublicValues, RandomSeed};
use zisk_coordinator_api::dto::{DomainInputChunk, DomainInputKind};
use zisk_coordinator_client::{CoordinatorClient, InputSenderPushAdapter, Job};

use crate::input_stream::{ZiskStream, GRPC_PUSH_URI};
use zisk_prover_backend::GuestProgram;

use crate::aggregate_proofs::{AggregateProofsRequest, AggregationInput};
use crate::lifecycle::{SetupTarget, UploadTarget};
use crate::recurser::Recurser;
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
    /// Connect the gateway and construct the base client. Shared by
    /// [`build`](Self::build) and [`build_ext`](Self::build_ext) so the
    /// connection path stays in one place.
    fn connect(self) -> Result<RemoteClient> {
        crate::client::ensure_single_instance();
        let gw = CoordinatorClient::connect(self.url, self.connect_timeout, self.request_timeout)
            .map_err(SdkError::backend)?;
        Ok(RemoteClient { gw })
    }

    /// Build the client.
    ///
    /// Returns the type fixed by the constructor: a [`RemoteClient`] via
    /// [`ProverClient::remote`](crate::ProverClient::remote), or an
    /// [`ZiskClient`](crate::ZiskClient) via [`ZiskClient::remote`](crate::ZiskClient::remote).
    pub fn build(self) -> Result<Out> {
        Ok(self.connect()?.into())
    }

    /// Build an **extended** remote client.
    ///
    /// # Returns
    ///
    /// [`RemoteClientExt`] — a wrapper around [`RemoteClient`] that overrides
    /// [`execute`](RemoteClient::execute) and [`prove`](RemoteClient::prove)
    /// to return builders that can carry extended features.
    pub fn build_ext(self) -> Result<RemoteClientExt> {
        Ok(RemoteClientExt { inner: self.connect()? })
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
        seed: Option<RandomSeed>,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<crate::prove::ProveResult>> {
        self.do_prove(program, stdin, hints, seed, executor, proof_kind, timeout, subs)
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        seed: Option<RandomSeed>,
        executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteResult>> {
        self.do_execute(program, stdin, hints, seed, executor, timeout, subs)
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

    fn run_upload_aggregation_program(&self, agg: &Recurser) -> Result<UploadResult> {
        self.do_upload_aggregation_program(agg)
    }

    fn run_setup_aggregation_program(
        &self,
        agg: &Recurser,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        self.do_setup_aggregation_program(agg, timeout, subs)
    }

    fn run_aggregate_proofs(
        &self,
        agg: &Recurser,
        proof_a: &Proof,
        proof_b: &Proof,
        free_a: &[u64],
        free_b: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<crate::prove::ProveResult>> {
        self.do_aggregate_proofs(
            agg,
            proof_a,
            proof_b,
            free_a,
            free_b,
            root_c_recurser_agg,
            timeout,
            subs,
        )
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

    /// Submit a setup request. Accepts either a [`GuestProgram`] or a
    /// [`Recurser`]; the latter dispatches a recurser-setup job to a worker
    /// (upload the spec first via [`RemoteClient::upload`]).
    #[must_use]
    pub fn setup<'a, T: Into<SetupTarget<'a>>>(&'a self, target: T) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, target.into())
    }

    /// Submit a ROM setup request for an already-uploaded program by `hash_id`.
    ///
    /// Skips upload — the coordinator must already hold the program's ELF (e.g. from
    /// a prior [`upload`](Self::upload)). Returns `ProgramNotFound` from the coordinator
    /// otherwise.
    ///
    /// Programs only: a `recurser_id` (as returned by uploading a [`Recurser`]) is not
    /// a valid `hash_id` here and fails with `ProgramNotFound` — recursers are set up
    /// via [`setup`](Self::setup) with the `Recurser` value, which is already id-keyed
    /// on the wire.
    #[must_use]
    pub fn setup_by_id(&self, hash_id: impl Into<String>) -> SetupByIdRequest<'_> {
        SetupByIdRequest::new(self, hash_id.into())
    }

    /// Upload/register a program or recurser with the coordinator. Recurser
    /// uploads push the aggregation-program spec; re-uploads are idempotent.
    #[must_use]
    pub fn upload<'a, T: Into<UploadTarget<'a>>>(&'a self, target: T) -> UploadRequest<'a, Self> {
        UploadRequest::new(self, target.into())
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

    /// Submit a recurser prove request; dispatched to a worker as an
    /// AggregateProofs job (upload + setup the recurser first).
    #[must_use]
    pub fn aggregate_proofs<'a>(
        &'a self,
        agg: &'a Recurser,
        input_a: impl Into<AggregationInput<'a>>,
        input_b: impl Into<AggregationInput<'a>>,
    ) -> AggregateProofsRequest<'a, Self> {
        AggregateProofsRequest::new(self, agg, input_a.into(), input_b.into())
    }
}

/// Extended remote client with opt-in, job-level metadata on `execute`/`prove`.
///
/// Built via [`RemoteClientBuilder::build_ext`]. Dereferences to [`RemoteClient`],
/// so every other operation (`setup`, `upload`, `wrap_proof`, `aggregate_proofs`,
/// …) is available unchanged; only [`execute`](Self::execute) and
/// [`prove`](Self::prove) are overridden to return metadata-capable builders.
#[derive(Clone)]
pub struct RemoteClientExt {
    inner: RemoteClient,
}

impl std::ops::Deref for RemoteClientExt {
    type Target = RemoteClient;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RemoteClientExt {
    /// Submit a prove request that can carry opaque job-level metadata via
    /// [`ProveRequestExt::metadata`](crate::ProveRequestExt::metadata).
    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> crate::prove::ProveRequestExt<'a> {
        crate::prove::ProveRequestExt::new(&self.inner, program, stdin)
    }

    /// Submit an execute request (dry-run, no proof) that can carry opaque
    /// job-level metadata via
    /// [`ExecuteRequestExt::metadata`](crate::ExecuteRequestExt::metadata).
    #[must_use]
    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> crate::execute::ExecuteRequestExt<'a> {
        crate::execute::ExecuteRequestExt::new(&self.inner, program, stdin)
    }
}

/// How a job's input, or its hints, reach the job once the job is submitted.
pub(crate) enum Delivery {
    /// In the request itself, or there is none.
    InRequest,
    /// The caller's stream. A gRPC one gets its sender once the job exists.
    Stream(ZiskStream),
    /// Data over the inline limit, pushed through the job's gRPC stream once the job exists:
    /// the caller does not see the difference.
    Push(Vec<u8>),
}

impl Delivery {
    /// Prepares the caller's stream, if any, before the job is submitted.
    pub(crate) fn start(&self) -> Result<()> {
        if let Self::Stream(stream) = self {
            stream.start()?;
        }
        Ok(())
    }

    /// Delivers what is left once `remote_job` exists, and returns the caller's stream for the
    /// job handle. `hints` selects the job's hints stream over its input stream.
    ///
    /// A push sends the data whole: the sender slices it into chunks of at most 3 MB, each copied
    /// only into its own gRPC message, and the coordinator forwards each chunk to the workers as
    /// it arrives, so nothing holds a second copy of the data. A push that fails cancels the
    /// job, which would otherwise wait for the rest of its input until its phase times out.
    pub(crate) fn complete(self, remote_job: &Job, hints: bool) -> Result<Option<ZiskStream>> {
        let open =
            || if hints { remote_job.open_hints_stream() } else { remote_job.open_input_stream() };
        match self {
            Self::InRequest => Ok(None),
            Self::Stream(stream) => {
                if stream.is_grpc() {
                    stream.set_input_sender(open());
                }
                Ok(Some(stream))
            }
            Self::Push(data) => {
                let sender = Box::new(InputSenderPushAdapter::new(open()));
                let pushed = sender.send_blocking(data).and_then(|()| sender.close_blocking());
                if let Err(e) = pushed {
                    let _ = remote_job.cancel();
                    return Err(SdkError::backend(e));
                }
                Ok(None)
            }
        }
    }
}

/// The most stdin bytes the guest's input region holds: 2 GB less its first word, the free
/// input, and its last one, as the ASM client checks an input file.
const MAX_STDIN_BYTES: usize = zisk_core::MAX_INPUT_SIZE as usize - 16;

/// Errors when `len` bytes of stdin do not fit the guest's input region: the worker refuses
/// them, and the job would fail only once the input has been sent.
fn ensure_fits_input_region(len: usize) -> Result<()> {
    if len > MAX_STDIN_BYTES {
        return Err(SdkError::backend(format!(
            "input is {len} bytes, more than the {MAX_STDIN_BYTES} bytes of the guest's input region"
        )));
    }
    Ok(())
}

/// The input kind of `data`: inline up to `inline_limit` bytes, a gRPC push stream above it.
fn inline_or_push(data: Vec<u8>, inline_limit: usize) -> (DomainInputKind, Delivery) {
    if data.len() <= inline_limit {
        (DomainInputKind::Inline(DomainInputChunk { data }), Delivery::InRequest)
    } else {
        (DomainInputKind::StreamUri(GRPC_PUSH_URI.to_string()), Delivery::Push(data))
    }
}

/// Converts an [`InputSource`] into a `DomainInputKind` for submission, and what is left to
/// deliver once the job exists.
///
/// All stream types (unix, quic, grpc) send StreamUri so workers start in streaming mode
/// (InputNull). For grpc://, the coordinator skips the relay — data arrives via PushJobInput
/// instead, which is also how an input over the inline limit travels.
pub(crate) fn stdin_to_input_kind(stdin: InputSource) -> Result<(DomainInputKind, Delivery)> {
    match stdin {
        InputSource::Stream(stream) => {
            Ok((DomainInputKind::StreamUri(stream.uri().to_string()), Delivery::Stream(stream)))
        }
        InputSource::Stdin(s) => {
            let data = s.into_inner().read_data();
            ensure_fits_input_region(data.len())?;
            Ok(inline_or_push(data, DomainInputKind::MAX_INLINE_BYTES))
        }
    }
}

/// Converts an optional [`HintsSource`] into a `DomainInputKind` for submission, and what is
/// left to deliver once the job exists.
///
/// - `HintsSource::Hints` → reads data and sends it inline, or pushes it over the inline limit
/// - `HintsSource::Stream` → `StreamUri(stream.uri())`
pub(crate) fn hints_to_input_kind(
    hints: Option<HintsSource>,
) -> Result<(Option<DomainInputKind>, Delivery)> {
    let hints = match hints {
        Some(h) => h,
        None => return Ok((None, Delivery::InRequest)),
    };
    match hints {
        HintsSource::Stream(stream) => Ok((
            Some(DomainInputKind::StreamUri(stream.uri().to_string())),
            Delivery::Stream(*stream),
        )),
        HintsSource::Hints(h) => {
            let mut source = h.into_inner();
            source.open().map_err(SdkError::backend)?;
            let mut data = Vec::new();
            while let Some(chunk) = source.next().map_err(SdkError::backend)? {
                data.extend(chunk);
            }
            source.close().map_err(SdkError::backend)?;
            let (kind, delivery) = inline_or_push(data, DomainInputKind::MAX_INLINE_BYTES);
            Ok((Some(kind), delivery))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_up_to_the_inline_limit_travels_in_the_request() {
        let (kind, delivery) = inline_or_push(vec![7; 16], 16);
        assert!(
            matches!(kind, DomainInputKind::Inline(DomainInputChunk { ref data }) if data.len() == 16)
        );
        assert!(matches!(delivery, Delivery::InRequest));
    }

    #[test]
    fn stdin_larger_than_the_input_region_is_refused() {
        ensure_fits_input_region(MAX_STDIN_BYTES).unwrap();
        let err = ensure_fits_input_region(MAX_STDIN_BYTES + 1).unwrap_err().to_string();
        assert!(err.contains("more than the"), "{err}");
    }

    #[test]
    fn data_over_the_inline_limit_is_pushed_whole() {
        let (kind, delivery) = inline_or_push(vec![7; 17], 16);
        assert!(matches!(kind, DomainInputKind::StreamUri(ref uri) if uri == GRPC_PUSH_URI));
        assert!(matches!(delivery, Delivery::Push(ref data) if data.len() == 17));
    }
}
