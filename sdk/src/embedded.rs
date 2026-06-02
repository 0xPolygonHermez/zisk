//! Embedded backend client

pub(crate) mod execute;
pub(crate) mod execute_only;
pub(crate) mod prove;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod verify_constraints;
pub(crate) mod wrap;

pub use execute_only::{EmbeddedExecuteOnlyBuilder, EmbeddedExecuteOnlyClient};

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::setup::SetupResult;
use anyhow::Result;
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
    Client, ExecutorKind,
};

const ERR_ASSEMBLY_NOT_ENABLED: &str =
    "Assembly executor not enabled — call .assembly() on the builder";

/// Builder for an embedded [`ProverClient`].
pub struct EmbeddedClientBuilder {
    executor: ExecutorKind,
    proof_kind: ProofKind,
    embedded_opts: EmbeddedOpts,
    gpu: bool,
    asm_options: Option<AsmOptions>,
    proving_key: Option<PathBuf>,
    proving_key_snark: Option<PathBuf>,
    no_aggregation: bool,
}

impl Default for EmbeddedClientBuilder {
    fn default() -> Self {
        Self {
            executor: ExecutorKind::Emulator,
            proof_kind: ProofKind::VadcopFinalMinimal,
            embedded_opts: EmbeddedOpts::default(),
            gpu: false,
            asm_options: None,
            proving_key: None,
            proving_key_snark: None,
            no_aggregation: false,
        }
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
}

impl WitnessBuilderExt for EmbeddedClientBuilder {
    fn no_aggregation(mut self) -> Self {
        self.no_aggregation = true;
        self
    }
}

impl EmbeddedClientBuilder {
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

    #[must_use]
    pub fn execute_only(self) -> EmbeddedExecuteOnlyBuilder {
        EmbeddedExecuteOnlyBuilder::from_parts(self.executor, self.asm_options)
    }

    /// Build the [`EmbeddedClient`].
    pub fn build(self) -> Result<EmbeddedClient> {
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
        if self.no_aggregation {
            backend_opts = backend_opts.no_aggregation();
        }
        if let Some(asm_opts) = self.asm_options {
            *backend_opts.asm_options_mut() = asm_opts;
        }
        let pk = ZiskPaths::get_proving_key(backend_opts.get_proving_key());
        let pk_snark = ZiskPaths::get_proving_key_snark(backend_opts.get_proving_key_snark());
        let prover = match self.executor {
            ExecutorKind::Emulator => Self::build_emu(pk, pk_snark, backend_opts, self.proof_kind)?,
            ExecutorKind::Assembly => Self::build_asm(pk, pk_snark, backend_opts, self.proof_kind)?,
        };
        Ok(EmbeddedClient { prover: Arc::new(prover), executor: self.executor })
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
        )?;
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
        )?;
        Ok(EmbeddedProver::Asm(ZiskProver::<Asm>::new(asm, backend_opts)))
    }
}

enum EmbeddedProver {
    Emu(ZiskProver<Emu>),
    Asm(ZiskProver<Asm>),
}

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
    fn default_executor(&self) -> ExecutorKind {
        self.executor
    }

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

impl EmbeddedClient {
    /// Submit a prove request.
    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, stdin)
    }

    /// Submit an execute request (dry-run, no proof).
    #[must_use]
    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
    ) -> ExecuteRequest<'a, Self> {
        ExecuteRequest::new(self, program, stdin)
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
