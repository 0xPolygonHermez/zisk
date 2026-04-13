use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use zisk_common::ZiskProgramVK;

use zisk_common::ProofMode;
use zisk_prover_backend::{AsmOptions, GuestProgram};

use crate::job_handle::SubscriberList;
use crate::{
    embedded::{self, EmbeddedClient, EmbeddedClientBuilder, EmbeddedClientConfig},
    execute::{ExecuteRequest, ExecuteResult},
    input::ProgramInput,
    opts::ProverOpts,
    proof::Proof,
    prove::ProveRequest,
    remote::{self, RemoteClient, RemoteClientBuilder, RemoteClientConfig},
    setup::SetupRequest,
    upload::UploadRequest,
    wrap::WrapRequest,
    Client, ExecutorKind, ZiskProofWithPublicValues, ZiskPublics,
};
use std::sync::Arc;
use std::time::Duration;

use crate::{JobHandle, ProofKind};

static PROVER_CLIENT_CREATED: AtomicBool = AtomicBool::new(false);

fn ensure_single_instance() {
    if PROVER_CLIENT_CREATED.swap(true, Ordering::AcqRel) {
        panic!(
            "A ProverClient already exists. Only one instance is allowed per process. \
             Store it in a shared location (e.g., Arc<ProverClient>) and reuse it."
        );
    }
}

/// Builder for [`ProverClient`].
pub struct ProverClientBuilder<B> {
    executor: ExecutorKind,
    prover_options: ProverOpts,
    proof_kind: ProofKind,
    gpu: bool,
    asm_options: Option<AsmOptions>,
    backend: B,
}

impl Default for ProverClientBuilder<EmbeddedClientConfig> {
    fn default() -> Self {
        ProverClient::embedded()
    }
}

/// Methods shared across all backends.
impl<B> ProverClientBuilder<B> {
    /// Set proof generation options (e.g. minimal memory mode).
    #[must_use]
    pub fn with_prover_options(mut self, opts: ProverOpts) -> Self {
        self.prover_options = opts;
        self
    }

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

    /// Set ASM-specific options.
    ///
    /// Only valid when using the Assembly executor. Calling `.build()` with these
    /// set on a non-Assembly client will panic.
    #[must_use]
    pub fn asm_options(mut self, opts: AsmOptions) -> Self {
        self.asm_options = Some(opts);
        self
    }
}

/// Methods specific to the embedded backend.
impl ProverClientBuilder<EmbeddedClientConfig> {
    /// Set the path to the proving key directory.
    #[must_use]
    pub fn proving_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.backend.proving_key = Some(path.into());
        self
    }

    /// Set the path to the PLONK proving key directory.
    #[must_use]
    pub fn proving_key_plonk(mut self, path: impl Into<PathBuf>) -> Self {
        self.backend.proving_key_snark = Some(path.into());
        self
    }

    /// Build the [`ProverClient`].
    pub fn build(self) -> Result<ProverClient> {
        if self.asm_options.is_some() && self.executor != ExecutorKind::Assembly {
            panic!(
                "asm_options were set but the executor is not Assembly. \
                 Call .assembly() on the builder before setting asm_options."
            );
        }
        ensure_single_instance();

        let mut builder = EmbeddedClientBuilder::new(self.backend)
            .executor(self.executor)
            .with_prover_options(self.prover_options);

        if self.gpu {
            builder = builder.gpu();
        }
        if self.proof_kind == ProofKind::Plonk {
            builder = builder.plonk();
        }
        if let Some(opts) = self.asm_options {
            builder = builder.asm_options(opts);
        }
        let client = builder.build()?;
        Ok(ProverClient { inner: Arc::new(BackendClient::Embedded(Arc::new(client))) })
    }
}

/// Methods specific to the remote backend.
impl ProverClientBuilder<RemoteClientConfig> {
    /// Set the connection timeout.
    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.backend.connect_timeout = timeout;
        self
    }

    /// Set the request timeout for individual operations.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.backend.request_timeout = timeout;
        self
    }

    /// Build the [`ProverClient`].
    pub fn build(self) -> Result<ProverClient> {
        ensure_single_instance();
        let client = RemoteClientBuilder::new(self.backend)
            .with_prover_options(self.prover_options)
            .build_sync()?;
        Ok(ProverClient { inner: Arc::new(BackendClient::Remote(Arc::new(client))) })
    }
}

pub(crate) enum BackendClient {
    Embedded(Arc<EmbeddedClient>),
    Remote(Arc<RemoteClient>),
}

/// Prover client. Runs proofs using local (embedded) or remote infrastructure.
///
/// Obtain via:
/// - `ProverClient::default()` — zero-config client (Emulator, no GPU)
/// - `ProverClient::embedded().build()` — full embedded configuration
/// - `ProverClient::remote(url).build()` — remote coordinator (future)
pub struct ProverClient {
    pub(crate) inner: Arc<BackendClient>,
}

impl ProverClient {
    // -- Builders --
    /// Returns a builder for the embedded (local) backend.
    #[must_use]
    pub fn embedded() -> ProverClientBuilder<EmbeddedClientConfig> {
        ProverClientBuilder {
            executor: ExecutorKind::Emulator,
            proof_kind: ProofKind::StarkMinimal,
            prover_options: ProverOpts::default(),
            backend: EmbeddedClientConfig::default(),
            gpu: false,
            asm_options: None,
        }
    }

    /// Returns a builder for the remote (distributed) backend.
    ///
    /// # Example
    /// ```ignore
    /// let client = ProverClient::remote("http://coordinator:50051").build()?;
    /// ```
    #[must_use]
    pub fn remote(url: impl Into<String>) -> ProverClientBuilder<RemoteClientConfig> {
        ProverClientBuilder {
            executor: ExecutorKind::Emulator,
            proof_kind: ProofKind::StarkMinimal,
            prover_options: ProverOpts::default(),
            backend: RemoteClientConfig { url: url.into(), ..Default::default() },
            gpu: false,
            asm_options: None,
        }
    }

    // -- Requests --
    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, input)
    }

    #[must_use]
    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ExecuteRequest<'a, Self> {
        ExecuteRequest::new(self, program, input)
    }

    #[must_use]
    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    #[must_use]
    pub fn upload<'a>(&'a self, program: &'a GuestProgram) -> UploadRequest<'a, Self> {
        UploadRequest::new(self, program)
    }

    #[must_use]
    pub fn wrap_proof<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
        mode: ProofMode,
    ) -> WrapRequest<'a, Self> {
        WrapRequest::new(self, proof_with_publics, mode)
    }
}

impl Clone for ProverClient {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl Default for ProverClient {
    fn default() -> Self {
        ProverClient::embedded().build().expect("Failed to initialize default ProverClient")
    }
}

impl Drop for ProverClient {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            PROVER_CLIENT_CREATED.store(false, Ordering::Release);
        }
    }
}

impl Client for ProverClient {
    fn run_upload(&self, program: &GuestProgram) -> Result<()> {
        match self.inner.as_ref() {
            BackendClient::Embedded(e) => embedded::upload::run(e, program),
            BackendClient::Remote(r) => remote::upload::run(r, program),
        }
    }

    fn run_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        timeout: Option<std::time::Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<()>> {
        match self.inner.as_ref() {
            BackendClient::Embedded(e) => {
                embedded::setup::run(Arc::clone(e), program, with_hints, timeout, subs)
            }
            BackendClient::Remote(r) => remote::setup::run(r, program, with_hints, timeout, subs),
        }
    }

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        timeout: Option<std::time::Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<Proof>> {
        match self.inner.as_ref() {
            BackendClient::Embedded(e) => {
                embedded::prove::run(Arc::clone(e), program, input, executor, mode, timeout, subs)
            }
            BackendClient::Remote(r) => {
                remote::prove::run(r, program, input, executor, mode, timeout, subs)
            }
        }
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        timeout: Option<std::time::Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteResult>> {
        match self.inner.as_ref() {
            BackendClient::Embedded(e) => {
                embedded::execute::run(Arc::clone(e), program, input, executor, timeout, subs)
            }
            BackendClient::Remote(r) => {
                remote::execute::run(r, program, input, executor, timeout, subs)
            }
        }
    }

    fn run_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        mode: ProofMode,
        override_publics: Option<ZiskPublics>,
        override_program_vk: Option<ZiskProgramVK>,
        timeout: Option<std::time::Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ZiskProofWithPublicValues>> {
        match self.inner.as_ref() {
            BackendClient::Embedded(e) => embedded::wrap::run(
                Arc::clone(e),
                proof_with_publics,
                mode,
                override_publics,
                override_program_vk,
                timeout,
                subs,
            ),
            BackendClient::Remote(r) => {
                remote::wrap::run(r, proof_with_publics, mode, timeout, subs)
            }
        }
    }
}
