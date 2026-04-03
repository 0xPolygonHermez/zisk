use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use proofman_common::ParamsGPU;
use zisk_common::ZiskProgramVK;

use zisk_common::ProofMode;
use zisk_prover_backend::{GuestProgram, ProofOpts};

use crate::{
    async_prove::AsyncProveRequest,
    cancel::CancellationToken,
    embedded::{EmbeddedClient, EmbeddedClientBuilder, EmbeddedClientConfig},
    execute::{ExecuteRequest, ExecuteResult},
    input::ProgramInput,
    plonk::PlonkRequest,
    proof::Proof,
    prove::ProveRequest,
    reduce::ReduceRequest,
    remote::{RemoteClient, RemoteClientBuilder, RemoteClientConfig},
    setup::SetupRequest,
    upload::UploadRequest,
    Client, ExecutorKind, ZiskProofWithPublicValues, ZiskPublics,
};
use std::sync::Arc;
use std::time::Duration;

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
///
/// Obtain via [`ProverClient::embedded`]. The type parameter `B` is the backend config
/// (`EmbeddedConfig`, or `RemoteClientConfig`) — it determines which methods
/// are available and which backend is constructed on `.build()`.
pub struct ProverClientBuilder<B> {
    executor: ExecutorKind,
    gpu_params: Option<ParamsGPU>,
    backend: B,
}

impl Default for ProverClientBuilder<EmbeddedClientConfig> {
    fn default() -> Self {
        ProverClient::embedded()
    }
}

/// Methods shared across all backends.
impl<B> ProverClientBuilder<B> {
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

    /// Enable GPU acceleration with default parameters.
    #[must_use]
    pub fn gpu(mut self) -> Self {
        self.gpu_params = Some(ParamsGPU::default());
        self
    }

    /// Enable GPU acceleration with custom parameters.
    #[must_use]
    pub fn with_gpu_params(mut self, gpu_params: ParamsGPU) -> Self {
        self.gpu_params = Some(gpu_params);
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

    /// Set the path to the SNARK proving key directory.
    #[must_use]
    pub fn proving_key_snark(mut self, path: impl Into<PathBuf>) -> Self {
        self.backend.proving_key_snark = Some(path.into());
        self
    }

    /// Build the [`ProverClient`].
    pub fn build(self) -> Result<ProverClient> {
        ensure_single_instance();
        let builder = EmbeddedClientBuilder::new(self.backend).executor(self.executor);
        let builder = match self.gpu_params {
            Some(params) => builder.with_gpu_params(params),
            None => builder,
        };
        let client = builder.build()?;
        Ok(ProverClient { inner: Arc::new(BackendClient::Embedded(client)) })
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
        let client = RemoteClientBuilder::new(self.backend).build_sync()?;
        Ok(ProverClient { inner: Arc::new(BackendClient::Remote(client)) })
    }
}

enum BackendClient {
    Embedded(EmbeddedClient),
    Remote(RemoteClient),
}

/// Prover client. Runs proofs using local (embedded) or remote infrastructure.
///
/// Obtain via:
/// - `ProverClient::default()` — zero-config client (Emulator, no GPU)
/// - `ProverClient::embedded().build()` — full embedded configuration
/// - `ProverClient::remote(url).build()` — remote coordinator (future)
pub struct ProverClient {
    inner: Arc<BackendClient>,
}

impl ProverClient {
    // -- Builders --
    /// Returns a builder for the embedded (local) backend.
    #[must_use]
    pub fn embedded() -> ProverClientBuilder<EmbeddedClientConfig> {
        ProverClientBuilder {
            executor: ExecutorKind::Emulator,
            gpu_params: None,
            backend: EmbeddedClientConfig::default(),
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
            gpu_params: None,
            backend: RemoteClientConfig { url: url.into(), ..Default::default() },
        }
    }

    // -- Requests --
    pub fn vk(&self, program: &GuestProgram) -> Result<ZiskProgramVK> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.vk(program),
            BackendClient::Remote(c) => c.vk(program),
        }
    }

    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, input)
    }

    /// Async variant of [`prove`](Self::prove).
    ///
    /// Returns an [`AsyncProveRequest`] builder. Call `.submit()` for non-blocking execution
    /// or `.run()` for blocking execution with event support.
    #[must_use]
    pub fn prove_async(
        &self,
        program: &GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> AsyncProveRequest<Self> {
        AsyncProveRequest::new(self.clone(), program.clone(), input)
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
    pub fn reduce<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
    ) -> ReduceRequest<'a, Self> {
        ReduceRequest::new(self, proof_with_publics)
    }

    #[must_use]
    pub fn plonk<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
    ) -> PlonkRequest<'a, Self> {
        PlonkRequest::new(self, proof_with_publics)
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
        // Only reset the singleton guard when the last clone is dropped.
        if Arc::strong_count(&self.inner) == 1 {
            PROVER_CLIENT_CREATED.store(false, Ordering::Release);
        }
    }
}

impl Client for ProverClient {
    fn run_upload(&self, program: &GuestProgram) -> Result<()> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.run_upload(program),
            BackendClient::Remote(c) => c.run_upload(program),
        }
    }

    fn run_setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.run_setup(program, with_hints),
            BackendClient::Remote(c) => c.run_setup(program, with_hints),
        }
    }

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        opts: ProofOpts,
        cancel: Option<&CancellationToken>,
    ) -> Result<Proof> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.run_prove(program, input, executor, mode, opts, cancel),
            BackendClient::Remote(c) => c.run_prove(program, input, executor, mode, opts, cancel),
        }
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        cancel: Option<&CancellationToken>,
    ) -> Result<ExecuteResult> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.run_execute(program, input, executor, cancel),
            BackendClient::Remote(c) => c.run_execute(program, input, executor, cancel),
        }
    }

    fn run_reduce(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => {
                c.run_reduce(proof_with_publics, override_publics, override_program_vk)
            }
            BackendClient::Remote(c) => {
                c.run_reduce(proof_with_publics, override_publics, override_program_vk)
            }
        }
    }

    fn run_plonk(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => {
                c.run_plonk(proof_with_publics, override_publics, override_program_vk)
            }
            BackendClient::Remote(c) => {
                c.run_plonk(proof_with_publics, override_publics, override_program_vk)
            }
        }
    }
}
