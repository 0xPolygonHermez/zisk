//! Embedded backend client

pub(crate) mod execute;
pub(crate) mod execute_only;
pub(crate) mod prove;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod verify_constraints;
pub(crate) mod wrap;

pub use execute_only::{EmbeddedExecuteOnlyBuilder, EmbeddedExecuteOnlyClient};

use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::setup::SetupResult;
use crate::{Result, SdkError};
use zisk_common::ProofKind;
use zisk_common::{ProgramVK, Proof, PublicValues, ZiskPaths};
use zisk_prover_backend::{Asm, AsmOptions, AsmProver, Emu, EmuProver, GuestProgram, ZiskProver};

use crate::{
    execute::{ExecuteRequest, ExecuteResult},
    hints::HintsSource,
    input_source::InputSource,
    job_handle::{JobHandle, SubscriberList},
    opts::EmbeddedOpts,
    prove::ProveRequest,
    setup::SetupRequest,
    upload::UploadRequest,
    wrap::WrapRequest,
    Client, ClientSync, ExecutorKind,
};

const ERR_ASSEMBLY_NOT_ENABLED: &str =
    "Assembly executor not enabled — call .assembly() on the builder";
const ERR_HINTS_REQUIRE_ASSEMBLY: &str = "Hints require Assembly executor";
const ERR_STREAM_STDIN_ON_EMULATOR: &str =
    "Stream stdin (quic://, unix://) is not supported with the Emulator executor — use Assembly executor";
const ERR_GRPC_ON_EMBEDDED: &str =
    "gRPC streams are not supported with the embedded executor — use a remote client";
const ERR_SETUP_WITHOUT_HINTS: &str =
    "Program was set up without hints — call setup().with_hints() first";
const ERR_SETUP_WITH_HINTS: &str = "Program was set up with hints — call .hints() on the request";

/// How a request's hints are delivered — the only distinction the
/// embedded-executor compatibility policy cares about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HintsKind {
    /// Inline (memory/file) hints.
    Inline,
    /// Local (unix/quic) stream.
    LocalStream,
    /// gRPC stream — never serviceable by the embedded executor.
    GrpcStream,
}

/// How a request's stdin is delivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StdinKind {
    /// Buffered (memory/file) input.
    Buffered,
    /// Local (unix/quic) stream.
    LocalStream,
    /// gRPC stream.
    GrpcStream,
}

impl HintsKind {
    fn of(hints: &HintsSource) -> Self {
        match hints {
            HintsSource::Hints(_) => HintsKind::Inline,
            HintsSource::Stream(s) if s.is_grpc() => HintsKind::GrpcStream,
            HintsSource::Stream(_) => HintsKind::LocalStream,
        }
    }
}

impl StdinKind {
    fn of(stdin: &InputSource) -> Self {
        match stdin {
            InputSource::Stdin(_) => StdinKind::Buffered,
            InputSource::Stream(s) if s.is_grpc() => StdinKind::GrpcStream,
            InputSource::Stream(_) => StdinKind::LocalStream,
        }
    }
}

/// Validate that an embedded execute/prove request is serviceable by the
/// configured executor, returning the same [`SdkError`] the dispatch would.
///
/// This is the single source of truth for the embedded compatibility policy,
/// factored out of `do_execute_inner` / `do_prove_inner` (which are identical)
/// so it can be exhaustively unit-tested without constructing a real prover.
/// `verify_constraints` keeps its own (slightly different) checks.
///
/// Error precedence is preserved exactly: for the `Emulator` executor, hints
/// are rejected before stream stdin; for `Assembly`, a missing/extra-hints
/// configuration error is reported before a gRPC-transport error.
pub(crate) fn validate_embedded_request(
    prover_is_asm: bool,
    executor: ExecutorKind,
    hints: Option<HintsKind>,
    stdin: StdinKind,
    was_setup_with_hints: bool,
) -> Result<()> {
    match executor {
        // The Emulator backend (whether the client was built `.emulator()` or
        // is an Assembly prover asked to run in emulator mode) accepts neither
        // hints nor a streamed stdin.
        ExecutorKind::Emulator => {
            if hints.is_some() {
                return Err(SdkError::UnsupportedExecutor(ERR_HINTS_REQUIRE_ASSEMBLY.to_string()));
            }
            if stdin != StdinKind::Buffered {
                return Err(SdkError::UnsupportedExecutor(
                    ERR_STREAM_STDIN_ON_EMULATOR.to_string(),
                ));
            }
            Ok(())
        }
        ExecutorKind::Assembly => {
            if !prover_is_asm {
                return Err(SdkError::UnsupportedExecutor(ERR_ASSEMBLY_NOT_ENABLED.to_string()));
            }
            match hints {
                Some(hints) => {
                    if !was_setup_with_hints {
                        return Err(SdkError::InvalidConfig(ERR_SETUP_WITHOUT_HINTS.to_string()));
                    }
                    if hints == HintsKind::GrpcStream {
                        return Err(SdkError::UnsupportedExecutor(
                            ERR_GRPC_ON_EMBEDDED.to_string(),
                        ));
                    }
                    Ok(())
                }
                None => {
                    if was_setup_with_hints {
                        return Err(SdkError::InvalidConfig(ERR_SETUP_WITH_HINTS.to_string()));
                    }
                    if stdin == StdinKind::GrpcStream {
                        return Err(SdkError::UnsupportedExecutor(
                            ERR_GRPC_ON_EMBEDDED.to_string(),
                        ));
                    }
                    Ok(())
                }
            }
        }
    }
}

/// Builder for an embedded client.
///
/// The `Out` type parameter selects what [`build`](Self::build) returns, and is fixed by the
/// constructor used:
/// - [`ProverClient::embedded`](crate::ProverClient::embedded) → `Out = EmbeddedClient` (the
///   concrete, fully-typed client).
/// - [`ZiskClient::embedded`](crate::ZiskClient::embedded) → `Out = ZiskClient` (the runtime-dispatch
///   façade).
///
/// The parameter is inferred at call sites and never needs to be named.
pub struct EmbeddedClientBuilder<Out = EmbeddedClient> {
    executor: ExecutorKind,
    proof_kind: ProofKind,
    embedded_opts: EmbeddedOpts,
    gpu: bool,
    asm_options: Option<AsmOptions>,
    proving_key: Option<PathBuf>,
    proving_key_snark: Option<PathBuf>,
    verbose: u8,
    no_aggregation: bool,
    verify_constraints: bool,
    _out: PhantomData<fn() -> Out>,
}

impl<Out> EmbeddedClientBuilder<Out> {
    /// Construct a builder with default settings for a given output type.
    ///
    /// Generic over `Out` so both [`ProverClient::embedded`](crate::ProverClient::embedded)
    /// (`Out = EmbeddedClient`) and [`ZiskClient::embedded`](crate::ZiskClient::embedded)
    /// (`Out = ZiskClient`) can construct one. Public construction goes through those entry points
    /// or [`Default`] (which is implemented only for `Out = EmbeddedClient`).
    pub(crate) fn for_output() -> Self {
        Self {
            executor: ExecutorKind::Emulator,
            proof_kind: ProofKind::VadcopFinalMinimal,
            embedded_opts: EmbeddedOpts::default(),
            gpu: false,
            asm_options: None,
            proving_key: None,
            proving_key_snark: None,
            verbose: 0,
            no_aggregation: false,
            verify_constraints: false,
            _out: PhantomData,
        }
    }
}

// Implemented only for the concrete output so `EmbeddedClientBuilder::default()` resolves
// unambiguously to `EmbeddedClientBuilder<EmbeddedClient>`. The `ZiskClient` variant is built via
// [`ZiskClient::embedded`](crate::ZiskClient::embedded), not `Default`.
impl Default for EmbeddedClientBuilder<EmbeddedClient> {
    fn default() -> Self {
        Self::for_output()
    }
}

/// Build-time extension that unlocks witness-only (no-aggregation) configuration on
/// [`EmbeddedClientBuilder`].
///
/// This is an import-gated extension trait: [`no_aggregation`](Self::no_aggregation) is only in
/// scope when this trait is imported, so a client can be built without the (expensive) aggregation
/// setup only when the caller has explicitly opted into a witness-generation workload — e.g.
/// [`verify_constraints`](crate::VerifyConstraintsExtension::verify_constraints) or `execute`.
pub trait WitnessBuilderExt: Sized {
    /// Skip aggregation setup when building the client.
    ///
    /// Witness-only workloads (`verify_constraints`, `execute`) never aggregate, so the aggregation
    /// circuits/keys that [`EmbeddedClientBuilder::build`] would otherwise set up in `ProofMan::new`
    /// are pure overhead for them. The resulting client is intended for those operations only —
    /// proof generation requires the aggregation setup this skips.
    #[must_use]
    fn no_aggregation(self) -> Self;

    /// Configure the client for constraint verification, a witness-only workload.
    #[must_use]
    fn verify_constraints(self) -> Self;
}

impl<Out> WitnessBuilderExt for EmbeddedClientBuilder<Out> {
    fn no_aggregation(mut self) -> Self {
        self.no_aggregation = true;
        self
    }

    fn verify_constraints(mut self) -> Self {
        self.verify_constraints = true;
        self
    }
}

impl<Out> EmbeddedClientBuilder<Out> {
    /// Set the executor kind. Default is [`ExecutorKind::Emulator`].
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = executor;
        self
    }

    /// Use the Emulator executor (default). Not compatible with hints.
    #[must_use]
    pub fn emulator(mut self) -> Self {
        self.executor = ExecutorKind::Emulator;
        self
    }

    /// Use the Assembly executor.
    #[must_use]
    pub fn assembly(mut self) -> Self {
        self.executor = ExecutorKind::Assembly;
        self
    }

    /// Set proof generation options (e.g. minimal memory mode).
    #[must_use]
    pub fn with_embedded_opts(mut self, opts: EmbeddedOpts) -> Self {
        self.embedded_opts = opts;
        self
    }

    /// Enable GPU acceleration.
    #[must_use]
    pub fn gpu(mut self) -> Self {
        self.gpu = true;
        self
    }

    /// Enable PLONK proof mode.
    #[must_use]
    pub fn plonk(mut self) -> Self {
        self.proof_kind = ProofKind::Plonk;
        self
    }

    /// Set ASM-specific options. Only valid with the Assembly executor.
    #[must_use]
    pub fn asm_options(mut self, opts: AsmOptions) -> Self {
        self.asm_options = Some(opts);
        self
    }

    /// Set the path to the proving key directory.
    #[must_use]
    pub fn proving_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key = Some(path.into());
        self
    }

    /// Set the path to the PLONK proving key directory.
    #[must_use]
    pub fn proving_key_plonk(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key_snark = Some(path.into());
        self
    }

    /// Set the prover verbosity level (`0` = quiet, higher = more verbose).
    #[must_use]
    pub fn verbose(mut self, level: u8) -> Self {
        self.verbose = level;
        self
    }

    /// Build a client that supports only `execute` (no proof generation).
    #[must_use]
    pub fn execute_only(self) -> EmbeddedExecuteOnlyBuilder {
        EmbeddedExecuteOnlyBuilder::from_parts(self.executor, self.asm_options)
    }

    fn build_emu(
        pk: PathBuf,
        pk_snark: PathBuf,
        backend_opts: zisk_prover_backend::BackendProverOpts,
        proof_kind: ProofKind,
    ) -> Result<EmbeddedProver> {
        let emu = EmuProver::new(
            proof_kind == ProofKind::Plonk,        // plonk
            backend_opts.preload_plonk(),          // preload_snark
            pk,                                    // proving_key
            pk_snark,                              // proving_key_snark
            true,                                  // shared_tables
            backend_opts.build_proofman_options(), // options
            None,                                  // logging_config
        )
        .map_err(SdkError::backend)?;
        Ok(EmbeddedProver::Emu(ZiskProver::<Emu>::new(emu, backend_opts)))
    }

    fn build_asm(
        pk: PathBuf,
        pk_snark: PathBuf,
        backend_opts: zisk_prover_backend::BackendProverOpts,
        proof_kind: ProofKind,
    ) -> Result<EmbeddedProver> {
        let asm_opts = backend_opts.asm_options();
        let asm = AsmProver::new(
            proof_kind == ProofKind::Plonk,        // plonk
            backend_opts.preload_plonk(),          // preload_snark
            pk,                                    // proving_key
            pk_snark,                              // proving_key_snark
            true,                                  // shared_tables
            asm_opts.unlock_mapped_memory,         // unlock_mapped_memory
            asm_opts.asm_out_file,                 // asm_out_file
            asm_opts.no_auto_setup,                // no_auto_setup
            backend_opts.build_proofman_options(), // options
            false,                                 // is_distributed
            None,                                  // logging_config
            backend_opts.cpu_mops_enabled(),       // cpu_mops
        )
        .map_err(SdkError::backend)?;
        Ok(EmbeddedProver::Asm(ZiskProver::<Asm>::new(asm, backend_opts)))
    }
}

impl<Out: From<EmbeddedClient>> EmbeddedClientBuilder<Out> {
    /// Build the client.
    ///
    /// Returns the type fixed by the constructor: an [`EmbeddedClient`] via
    /// [`ProverClient::embedded`](crate::ProverClient::embedded), or an
    /// [`ZiskClient`](crate::ZiskClient) via [`ZiskClient::embedded`](crate::ZiskClient::embedded).
    pub fn build(self) -> Result<Out> {
        crate::client::ensure_single_instance();
        if self.asm_options.is_some() && self.executor != ExecutorKind::Assembly {
            panic!(
                "asm_options were set but the executor is not Assembly. \
                 Call .assembly() on the builder before setting asm_options."
            );
        }
        let mut embedded_opts = self.embedded_opts;
        if let Some(pk) = self.proving_key {
            embedded_opts.proving_key = Some(pk);
        }
        if let Some(pk) = self.proving_key_snark {
            embedded_opts.proving_key_snark = Some(pk);
        }
        let mut backend_opts = embedded_opts.into_backend_opts(self.gpu);
        if self.verbose > 0 {
            backend_opts = backend_opts.verbose(self.verbose);
        }
        if self.no_aggregation {
            backend_opts = backend_opts.no_aggregation();
        }
        if self.verify_constraints {
            backend_opts = backend_opts.verify_constraints();
        }
        if let Some(asm_opts) = self.asm_options {
            *backend_opts.asm_options_mut() = asm_opts;
        }
        let pk = ZiskPaths::get_proving_key(backend_opts.get_proving_key());
        let pk_snark = ZiskPaths::get_proving_key_snark(backend_opts.get_proving_key_snark());
        let prover = match self.executor {
            ExecutorKind::Emulator => EmbeddedClientBuilder::<Out>::build_emu(
                pk,
                pk_snark,
                backend_opts,
                self.proof_kind,
            )?,
            ExecutorKind::Assembly => EmbeddedClientBuilder::<Out>::build_asm(
                pk,
                pk_snark,
                backend_opts,
                self.proof_kind,
            )?,
        };
        Ok(EmbeddedClient { prover: Arc::new(prover), executor: self.executor }.into())
    }
}

enum EmbeddedProver {
    Emu(ZiskProver<Emu>),
    Asm(ZiskProver<Asm>),
}

/// Embedded client implementation.
pub struct EmbeddedClient {
    prover: Arc<EmbeddedProver>,
    executor: ExecutorKind,
}

impl Clone for EmbeddedClient {
    fn clone(&self) -> Self {
        Self { prover: Arc::clone(&self.prover), executor: self.executor }
    }
}

impl Client for EmbeddedClient {
    fn run_upload(&self, program: &GuestProgram) -> Result<crate::upload::UploadResult> {
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
        override_publics: Option<PublicValues>,
        override_program_vk: Option<ProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<crate::prove::ProveResult>> {
        self.do_wrap(proof, proof_kind, override_publics, override_program_vk, timeout, subs)
    }
}

impl ClientSync for EmbeddedClient {
    fn run_setup_sync(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
        subs: SubscriberList,
    ) -> Result<SetupResult> {
        self.do_setup_sync(program, with_hints, emulator_only, subs)
    }

    fn run_prove_sync(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        subs: SubscriberList,
    ) -> Result<crate::prove::ProveResult> {
        self.do_prove_sync(program, stdin, hints, executor, proof_kind, subs)
    }

    fn run_execute_sync(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        subs: SubscriberList,
    ) -> Result<ExecuteResult> {
        self.do_execute_sync(program, stdin, hints, executor, subs)
    }

    fn run_wrap_sync(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<PublicValues>,
        override_program_vk: Option<ProgramVK>,
        subs: SubscriberList,
    ) -> Result<crate::prove::ProveResult> {
        self.do_wrap_sync(proof, proof_kind, override_publics, override_program_vk, subs)
    }
}

impl EmbeddedClient {
    /// The executor this client was built with.
    ///
    /// Crate-internal: used by [`ZiskClient`](crate::ZiskClient) to forward the configured executor
    /// into `prove`/`execute` requests, mirroring the behavior of this client's own request
    /// builders. Not part of the public surface — `RemoteClient` selects an executor per request
    /// and has no single value to mirror.
    #[must_use]
    pub(crate) fn executor(&self) -> ExecutorKind {
        self.executor
    }

    /// Submit a prove request.
    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, stdin, self.executor)
    }

    /// Submit an execute request (dry-run, no proof).
    #[must_use]
    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> ExecuteRequest<'a, Self> {
        ExecuteRequest::new(self, program, stdin, self.executor)
    }

    /// Submit a ROM setup request.
    #[must_use]
    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    /// Submit an upload request (no-op for embedded — program is available locally).
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

#[cfg(test)]
mod validate_tests {
    use super::*;
    use crate::ExecutorKind::{Assembly, Emulator};

    // Convenience: assert a validation result is the expected error variant
    // carrying the expected message.
    fn assert_unsupported(res: Result<()>, msg: &str) {
        match res {
            Err(SdkError::UnsupportedExecutor(m)) => assert_eq!(m, msg),
            other => panic!("expected UnsupportedExecutor({msg:?}), got {other:?}"),
        }
    }
    fn assert_invalid(res: Result<()>, msg: &str) {
        match res {
            Err(SdkError::InvalidConfig(m)) => assert_eq!(m, msg),
            other => panic!("expected InvalidConfig({msg:?}), got {other:?}"),
        }
    }

    // ── Emulator executor (prover-agnostic: same policy for Emu and Asm) ──

    #[test]
    fn emulator_buffered_no_hints_ok() {
        for is_asm in [false, true] {
            assert!(validate_embedded_request(is_asm, Emulator, None, StdinKind::Buffered, false)
                .is_ok());
        }
    }

    #[test]
    fn emulator_rejects_hints_before_stream_stdin() {
        // Hints present + stream stdin → hints error wins (checked first).
        for is_asm in [false, true] {
            assert_unsupported(
                validate_embedded_request(
                    is_asm,
                    Emulator,
                    Some(HintsKind::Inline),
                    StdinKind::LocalStream,
                    false,
                ),
                ERR_HINTS_REQUIRE_ASSEMBLY,
            );
        }
    }

    #[test]
    fn emulator_rejects_stream_stdin() {
        for stdin in [StdinKind::LocalStream, StdinKind::GrpcStream] {
            assert_unsupported(
                validate_embedded_request(true, Emulator, None, stdin, false),
                ERR_STREAM_STDIN_ON_EMULATOR,
            );
        }
    }

    // ── Assembly executor on a non-Assembly (Emu) prover ──

    #[test]
    fn assembly_on_emu_prover_rejected_regardless_of_inputs() {
        for hints in [None, Some(HintsKind::Inline), Some(HintsKind::GrpcStream)] {
            for stdin in [StdinKind::Buffered, StdinKind::LocalStream, StdinKind::GrpcStream] {
                assert_unsupported(
                    validate_embedded_request(false, Assembly, hints, stdin, true),
                    ERR_ASSEMBLY_NOT_ENABLED,
                );
            }
        }
    }

    // ── Assembly executor on an Assembly prover, WITH hints ──

    #[test]
    fn assembly_hints_setup_with_hints_ok() {
        for h in [HintsKind::Inline, HintsKind::LocalStream] {
            assert!(validate_embedded_request(true, Assembly, Some(h), StdinKind::Buffered, true)
                .is_ok());
        }
    }

    #[test]
    fn assembly_hints_but_setup_without_hints_is_invalid_config() {
        assert_invalid(
            validate_embedded_request(
                true,
                Assembly,
                Some(HintsKind::Inline),
                StdinKind::Buffered,
                false,
            ),
            ERR_SETUP_WITHOUT_HINTS,
        );
    }

    #[test]
    fn assembly_grpc_hints_rejected() {
        assert_unsupported(
            validate_embedded_request(
                true,
                Assembly,
                Some(HintsKind::GrpcStream),
                StdinKind::Buffered,
                true,
            ),
            ERR_GRPC_ON_EMBEDDED,
        );
    }

    #[test]
    fn assembly_setup_error_takes_precedence_over_grpc() {
        // gRPC hints + setup-without-hints → the config error wins.
        assert_invalid(
            validate_embedded_request(
                true,
                Assembly,
                Some(HintsKind::GrpcStream),
                StdinKind::Buffered,
                false,
            ),
            ERR_SETUP_WITHOUT_HINTS,
        );
    }

    // ── Assembly executor on an Assembly prover, WITHOUT hints ──

    #[test]
    fn assembly_no_hints_ok_for_buffered_and_local_stream() {
        for stdin in [StdinKind::Buffered, StdinKind::LocalStream] {
            assert!(validate_embedded_request(true, Assembly, None, stdin, false).is_ok());
        }
    }

    #[test]
    fn assembly_no_hints_but_setup_with_hints_is_invalid_config() {
        assert_invalid(
            validate_embedded_request(true, Assembly, None, StdinKind::Buffered, true),
            ERR_SETUP_WITH_HINTS,
        );
    }

    #[test]
    fn assembly_no_hints_grpc_stdin_rejected() {
        assert_unsupported(
            validate_embedded_request(true, Assembly, None, StdinKind::GrpcStream, false),
            ERR_GRPC_ON_EMBEDDED,
        );
    }

    #[test]
    fn assembly_no_hints_setup_error_takes_precedence_over_grpc_stdin() {
        // gRPC stdin + setup-with-hints → the config error wins.
        assert_invalid(
            validate_embedded_request(true, Assembly, None, StdinKind::GrpcStream, true),
            ERR_SETUP_WITH_HINTS,
        );
    }
}
