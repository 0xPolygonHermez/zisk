//! Remote backend client — connects to a ZisK Coordinator for distributed proving.

pub(crate) mod execute;
pub(crate) mod prove;
pub(crate) mod recurser;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod wrap;

use crate::snark::{ensure_snark_proof, SnarkProtocol};
use crate::{Result, SdkError};
use std::collections::BTreeMap;
use std::time::Duration;
use zisk_common::io::StreamRead;
use zisk_common::{ProgramVK, Proof, ProofKind, PublicValues, RandomSeed};
use zisk_coordinator_api::dto::{
    DomainInputChunk, DomainInputKind, DomainJobKindResponse, TerminalStatus,
};
use zisk_coordinator_client::{CoordinatorClient, Job};

use crate::input_stream::{ZiskStream, GRPC_PUSH_URI};
use zisk_prover_backend::GuestProgram;

use crate::aggregate_proofs::{AggregateProofsRequest, AggregationInput};
use crate::lifecycle::{SetupTarget, UploadTarget};
use crate::recurser::Recurser;
use crate::{
    execute::{ExecuteRequest, ExecuteResult},
    hints::HintsSource,
    input_source::InputSource,
    job_handle::{InputPushes, JobHandle, SubscriberList},
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
    snark: Option<SnarkProtocol>,
    _out: std::marker::PhantomData<fn() -> Out>,
}

impl<Out> RemoteClientBuilder<Out> {
    pub(crate) fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            snark: None,
            _out: std::marker::PhantomData,
        }
    }

    /// Ask for final SNARK proofs of `protocol`, wrapped with [`ProofKind::Snark`]. The workers'
    /// SNARK proving key sets the protocol up: the coordinator refuses a job when theirs is of
    /// another, and a final SNARK proof of another still fails the job's result. Unset, the proof
    /// is the workers' SNARK.
    #[must_use]
    pub fn snark(mut self, protocol: SnarkProtocol) -> Self {
        self.snark = Some(protocol);
        self
    }

    /// Ask for PLONK final SNARK proofs: [`snark`](Self::snark) of [`SnarkProtocol::Plonk`].
    #[must_use]
    pub fn plonk(self) -> Self {
        self.snark(SnarkProtocol::Plonk)
    }

    /// Ask for pilfflonk final SNARK proofs: [`snark`](Self::snark) of
    /// [`SnarkProtocol::Pilfflonk`].
    #[must_use]
    pub fn pilfflonk(self) -> Self {
        self.snark(SnarkProtocol::Pilfflonk)
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
        Ok(RemoteClient { gw, snark: self.snark, legacy_ext: false })
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
    /// [`RemoteClientExt`] — a wrapper around [`RemoteClient`] whose `prove` and
    /// `execute` keep sending their metadata over the extended API, which
    /// coordinators that predate metadata on `JobRequest` also take.
    #[deprecated(
        since = "1.4.0-alpha",
        note = "build a `RemoteClient` with `ProverClient::remote(url).build()` (or reach it \
                with `ZiskClient::as_remote`): its requests take `metadata`; removed in the \
                first non-alpha release"
    )]
    #[allow(deprecated)]
    pub fn build_ext(self) -> Result<RemoteClientExt> {
        Ok(RemoteClientExt { inner: RemoteClient { legacy_ext: true, ..self.connect()? } })
    }
}

/// Remote client implementation.
#[derive(Clone)]
pub struct RemoteClient {
    pub(crate) gw: CoordinatorClient,
    /// The protocol final SNARK proofs must be of, if asked for.
    pub(crate) snark: Option<SnarkProtocol>,
    /// Built by the deprecated [`RemoteClientBuilder::build_ext`]: prove and execute
    /// metadata goes over the extended API's `JobRequestExt`, as it did before it moved
    /// onto `JobRequest`.
    pub(crate) legacy_ext: bool,
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
        metadata: BTreeMap<String, String>,
    ) -> Result<JobHandle<crate::prove::ProveResult>> {
        self.do_prove(program, stdin, hints, seed, executor, proof_kind, timeout, subs, metadata)
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
        metadata: BTreeMap<String, String>,
    ) -> Result<JobHandle<ExecuteResult>> {
        self.do_execute(program, stdin, hints, seed, executor, timeout, subs, metadata)
    }

    fn run_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        _override_publics: Option<PublicValues>,
        _override_program_vk: Option<ProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
        metadata: BTreeMap<String, String>,
    ) -> Result<JobHandle<crate::prove::ProveResult>> {
        self.do_wrap(proof, proof_kind, timeout, subs, metadata)
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
        metadata: BTreeMap<String, String>,
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
            metadata,
        )
    }
}

/// The protocol a job of `proof_kind` asks the coordinator for, of the client's `snark`: a final
/// SNARK's only.
pub(crate) fn requested_snark(
    snark: Option<SnarkProtocol>,
    proof_kind: ProofKind,
) -> Option<SnarkProtocol> {
    snark.filter(|_| proof_kind == ProofKind::Snark)
}

/// Fail a remote proof that is not a final SNARK of `protocol`. The coordinator sends
/// `Proof::to_bincode`, not the saved format `Proof::from_bytes` reads.
fn ensure_remote_snark_proof(protocol: SnarkProtocol, data: &[u8]) -> Result<()> {
    let proof = Proof::from_bincode(data)
        .map_err(|e| SdkError::Serialization(format!("failed to deserialize remote proof: {e}")))?;
    ensure_snark_proof(protocol, &proof)
}

impl RemoteClient {
    /// On completion, fail a final SNARK proof that is not of the protocol this client asks for.
    pub(crate) fn check_snark(
        &self,
        handle: &mut JobHandle<crate::prove::ProveResult>,
        proof_kind: ProofKind,
    ) {
        let Some(protocol) = requested_snark(self.snark, proof_kind) else {
            return;
        };
        handle.set_pre_process(move |status: &TerminalStatus| match status {
            TerminalStatus::Completed(DomainJobKindResponse::Prove { proof, .. })
            | TerminalStatus::Completed(DomainJobKindResponse::Wrap(proof)) => {
                ensure_remote_snark_proof(protocol, &proof.data)
            }
            _ => Ok(()),
        });
    }

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

/// Extended remote client of the deprecated metadata API.
///
/// Built via [`RemoteClientBuilder::build_ext`]. Dereferences to [`RemoteClient`],
/// whose `prove`, `execute`, `wrap_proof` and `aggregate_proofs` builders take
/// job `metadata`; `prove` and `execute` send it over the extended API, as before.
#[deprecated(
    since = "1.4.0-alpha",
    note = "use `RemoteClient`: its requests take `metadata`; removed in the first non-alpha release"
)]
#[derive(Clone)]
pub struct RemoteClientExt {
    inner: RemoteClient,
}

#[allow(deprecated)]
impl std::ops::Deref for RemoteClientExt {
    type Target = RemoteClient;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[allow(deprecated)]
impl RemoteClientExt {
    /// Submit a prove request that can carry job-level metadata.
    #[deprecated(
        since = "1.4.0-alpha",
        note = "use `RemoteClient::prove`; removed in the first non-alpha release"
    )]
    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> crate::prove::ProveRequestExt<'a> {
        self.inner.prove(program, stdin)
    }

    /// Submit an execute request (dry-run, no proof) that can carry job-level metadata.
    #[deprecated(
        since = "1.4.0-alpha",
        note = "use `RemoteClient::execute`; removed in the first non-alpha release"
    )]
    #[must_use]
    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> crate::execute::ExecuteRequestExt<'a> {
        self.inner.execute(program, stdin)
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
    /// job handle to finish. `hints` selects the job's hints stream over its input stream.
    ///
    /// A push runs in `pushes`, in the background: the caller gets the job handle at once, and a
    /// caller that feeds another stream of the job can only do so once it has it. The sender
    /// slices the data into chunks of at most 3 MB, each copied only into its own gRPC message,
    /// and the coordinator forwards each chunk to the workers as it arrives, so nothing holds a
    /// second copy of the data.
    pub(crate) fn complete(
        self,
        remote_job: &Job,
        hints: bool,
        pushes: &mut InputPushes,
    ) -> Option<ZiskStream> {
        let open =
            || if hints { remote_job.open_hints_stream() } else { remote_job.open_input_stream() };
        match self {
            Self::InRequest => None,
            Self::Stream(stream) => {
                if stream.is_grpc() {
                    stream.set_input_sender(open());
                }
                Some(stream)
            }
            Self::Push(data) => {
                pushes.spawn(
                    open(),
                    data,
                    remote_job.clone(),
                    if hints { "hints" } else { "input" },
                );
                None
            }
        }
    }
}

/// The most stdin bytes the guest's input region holds, as the coordinator also checks.
const MAX_STDIN_BYTES: usize = zisk_core::MAX_STDIN_SIZE as usize;

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

/// A job's input, or its hints, as the caller gave it: a stream, or data read whole.
enum Payload {
    Stream(ZiskStream),
    Data(Vec<u8>),
}

impl Payload {
    fn data_len(&self) -> Option<usize> {
        match self {
            Self::Stream(_) => None,
            Self::Data(data) => Some(data.len()),
        }
    }

    /// The input kind for the request, and what is left to deliver once the job exists. A
    /// stream sends its URI, so that the workers start in streaming mode (InputNull); data travels
    /// inline when `inline`, else through the job's gRPC push stream.
    fn into_kind(self, inline: bool) -> (DomainInputKind, Delivery) {
        match self {
            Self::Stream(stream) => {
                (DomainInputKind::StreamUri(stream.uri().to_string()), Delivery::Stream(stream))
            }
            Self::Data(data) if inline => {
                (DomainInputKind::Inline(DomainInputChunk { data }), Delivery::InRequest)
            }
            Self::Data(data) => {
                (DomainInputKind::StreamUri(GRPC_PUSH_URI.to_string()), Delivery::Push(data))
            }
        }
    }
}

/// Whether a job's input data travels inline within `budget` bytes, which a request holds for its
/// input and hints together, by their lengths (`None` for a stream). Hints data always travels
/// inline, and is refused past the budget: the coordinator buffers a pushed hints stream without
/// a bound, so only the input is pushed, when it does not fit beside the hints.
fn input_inline(input: Option<usize>, hints: Option<usize>, budget: usize) -> Result<bool> {
    let hints = hints.unwrap_or(0);
    if hints > budget {
        return Err(SdkError::backend(format!(
            "hints are {hints} bytes, more than the {budget} bytes a request carries inline"
        )));
    }
    Ok(input.is_some_and(|n| n + hints <= budget))
}

/// A job's input and hints for its request, and what is left to deliver of each once the job
/// exists.
pub(crate) struct JobInputs {
    pub(crate) input: DomainInputKind,
    pub(crate) input_delivery: Delivery,
    pub(crate) hints: Option<DomainInputKind>,
    pub(crate) hints_delivery: Delivery,
}

/// Converts a job's [`InputSource`] and optional [`HintsSource`] for its request. Hints data
/// travels inline, within [`DomainInputKind::MAX_INLINE_BYTES`]; the input data too when it fits
/// beside them, and through the job's gRPC push stream past that. For a grpc:// stream the
/// coordinator relays nothing: its data arrives through PushJobInput too.
pub(crate) fn job_inputs(stdin: InputSource, hints: Option<HintsSource>) -> Result<JobInputs> {
    let input = match stdin {
        InputSource::Stream(stream) => Payload::Stream(stream),
        InputSource::Stdin(s) => {
            let data = s.into_inner().into_data();
            ensure_fits_input_region(data.len())?;
            Payload::Data(data)
        }
    };
    let hints = hints.map(read_hints).transpose()?;

    let inline = input_inline(
        input.data_len(),
        hints.as_ref().and_then(Payload::data_len),
        DomainInputKind::MAX_INLINE_BYTES,
    )?;
    let (input, input_delivery) = input.into_kind(inline);
    let (hints, hints_delivery) = match hints {
        Some(hints) => {
            let (kind, delivery) = hints.into_kind(true);
            (Some(kind), delivery)
        }
        None => (None, Delivery::InRequest),
    };
    Ok(JobInputs { input, input_delivery, hints, hints_delivery })
}

/// A job's hints as the caller gave them: a stream, or the data, read whole.
fn read_hints(hints: HintsSource) -> Result<Payload> {
    match hints {
        HintsSource::Stream(stream) => Ok(Payload::Stream(*stream)),
        HintsSource::Hints(h) => {
            let mut source = h.into_inner();
            source.open().map_err(SdkError::backend)?;
            let mut data = Vec::new();
            while let Some(chunk) = source.next().map_err(SdkError::backend)? {
                data.extend(chunk);
            }
            source.close().map_err(SdkError::backend)?;
            Ok(Payload::Data(data))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_final_snark_job_asks_for_the_client_protocol() {
        let pilfflonk = Some(SnarkProtocol::Pilfflonk);
        assert_eq!(requested_snark(pilfflonk, ProofKind::Snark), pilfflonk);
        assert_eq!(requested_snark(None, ProofKind::Snark), None);
        for stark in [ProofKind::VadcopFinal, ProofKind::VadcopFinalMinimal] {
            assert_eq!(requested_snark(pilfflonk, stark), None);
        }
    }

    /// The protocol check reads the coordinator's bincode, the format it sends.
    #[test]
    fn a_remote_snark_proof_is_read_as_the_coordinator_sends_it() {
        let pilfflonk = Proof {
            body: zisk_common::ProofBody::Pilfflonk {
                proof_bytes: Vec::new(),
                vkey_json: String::new(),
                publics: Default::default(),
                publics_full: Vec::new(),
                rootc: Vec::new(),
            },
            ..Default::default()
        };
        let data = pilfflonk.to_bincode().unwrap();
        ensure_remote_snark_proof(SnarkProtocol::Pilfflonk, &data).unwrap();
        assert!(ensure_remote_snark_proof(SnarkProtocol::Plonk, &data).is_err());
        assert!(ensure_remote_snark_proof(SnarkProtocol::Pilfflonk, &[1, 2, 3]).is_err());
    }

    #[test]
    fn data_that_fits_the_budget_travels_in_the_request() {
        assert!(input_inline(Some(16), None, 16).unwrap());
        assert!(!input_inline(None, Some(16), 16).unwrap());
        assert!(input_inline(Some(8), Some(8), 16).unwrap());
        let (kind, delivery) = Payload::Data(vec![7; 16]).into_kind(true);
        assert!(
            matches!(kind, DomainInputKind::Inline(DomainInputChunk { ref data }) if data.len() == 16)
        );
        assert!(matches!(delivery, Delivery::InRequest));
    }

    /// A request carries its input and its hints together: when both do not fit, the input is
    /// pushed, whichever is larger, and the hints stay inline.
    #[test]
    fn the_input_is_pushed_when_input_and_hints_do_not_fit_together() {
        assert!(!input_inline(Some(10), Some(8), 16).unwrap());
        assert!(!input_inline(Some(8), Some(10), 16).unwrap());
        assert!(!input_inline(Some(17), None, 16).unwrap());
    }

    /// The coordinator buffers a pushed hints stream without a bound, so hints are never pushed.
    #[test]
    fn hints_over_the_budget_are_refused() {
        let err = input_inline(Some(1), Some(17), 16).unwrap_err().to_string();
        assert!(err.contains("hints are 17 bytes"), "{err}");
        assert!(input_inline(None, Some(17), 16).is_err());
    }

    #[test]
    fn data_over_the_budget_is_pushed_whole() {
        let (kind, delivery) = Payload::Data(vec![7; 17]).into_kind(false);
        assert!(matches!(kind, DomainInputKind::StreamUri(ref uri) if uri == GRPC_PUSH_URI));
        assert!(matches!(delivery, Delivery::Push(ref data) if data.len() == 17));
    }

    #[test]
    fn stdin_larger_than_the_input_region_is_refused() {
        ensure_fits_input_region(MAX_STDIN_BYTES).unwrap();
        let err = ensure_fits_input_region(MAX_STDIN_BYTES + 1).unwrap_err().to_string();
        assert!(err.contains("more than the"), "{err}");
    }
}
