use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use zisk_common::ZiskProgramVK;

use zisk_common::ProofMode;
use zisk_prover_backend::{GuestProgram, ProofOpts};

use crate::{
    async_prove::AsyncProveRequest,
    embedded::{EmbeddedClient, EmbeddedClientBuilder, EmbeddedOptions},
    execute::{ExecuteRequest, ExecuteResult},
    input::ProgramInput,
    plonk::PlonkRequest,
    proof::Proof,
    prove::ProveRequest,
    reduce::ReduceRequest,
    setup::SetupRequest,
    upload::UploadRequest,
    Client, ExecutorKind, ZiskProofWithPublicValues, ZiskPublics,
};

static PROVER_CLIENT_CREATED: AtomicBool = AtomicBool::new(false);

fn ensure_single_instance() {
    if PROVER_CLIENT_CREATED.swap(true, Ordering::AcqRel) {
        panic!(
            "A ProverClient already exists. Only one instance is allowed per process. \
             Store it in a shared location (e.g., Arc<ProverClient>) and reuse it."
        );
    }
}

enum BackendClient {
    Embedded(EmbeddedClient),
    // Remote(RemoteClient),
}

/// Prover client. Runs proofs using local (embedded) or remote infrastructure.
///
/// Obtain via:
/// - `ProverClient::default()` — zero-config embedded client (Emulator, no GPU)
/// - `ProverClient::embedded(opts).build()` — full embedded configuration
/// - `ProverClient::remote(url).build()` — remote coordinator (future)
pub struct ProverClient {
    inner: Arc<BackendClient>,
    cancel_fn: Arc<dyn Fn() + Send + Sync>,
}

impl ProverClient {
    pub(crate) fn from_embedded(client: EmbeddedClient) -> Self {
        ensure_single_instance();
        let inner = Arc::new(BackendClient::Embedded(client));
        let cancel_fn: Arc<dyn Fn() + Send + Sync> = {
            let i = Arc::clone(&inner);
            Arc::new(move || {
                if let BackendClient::Embedded(c) = i.as_ref() {
                    c.cancel();
                }
            })
        };
        Self { inner, cancel_fn }
    }

    pub fn embedded(options: EmbeddedOptions) -> EmbeddedClientBuilder {
        EmbeddedClientBuilder::new(options)
    }

    // pub fn remote(url: impl Into<String>) -> RemoteClientBuilder {
    //     ensure_single_instance();
    //     RemoteClientBuilder::new(url.into())
    // }

    pub fn vk(&self, program: &GuestProgram) -> Result<ZiskProgramVK> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.vk(program),
        }
    }

    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, input).with_cancel_fn(Arc::clone(&self.cancel_fn))
    }

    /// Async variant of [`prove`](Self::prove). Requires the client to be wrapped in [`Arc`].
    ///
    /// Returns an [`AsyncProveRequest`] builder. Call `.submit()` for non-blocking execution
    /// or `.run()` for blocking execution with event support.
    pub fn prove_async(
        self: &Arc<Self>,
        program: &GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> AsyncProveRequest<Arc<Self>> {
        AsyncProveRequest::new(Arc::clone(self), Arc::new(program.clone()), input)
            .with_cancel_fn(Arc::clone(&self.cancel_fn))
    }

    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ExecuteRequest<'a, Self> {
        ExecuteRequest::new(self, program, input).with_cancel_fn(Arc::clone(&self.cancel_fn))
    }

    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    pub fn upload<'a>(&'a self, program: &'a GuestProgram) -> UploadRequest<'a, Self> {
        UploadRequest::new(self, program)
    }

    pub fn reduce<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
    ) -> ReduceRequest<'a, Self> {
        ReduceRequest::new(self, proof_with_publics)
    }

    pub fn plonk<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
    ) -> PlonkRequest<'a, Self> {
        PlonkRequest::new(self, proof_with_publics)
    }
}

impl Clone for ProverClient {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner), cancel_fn: Arc::clone(&self.cancel_fn) }
    }
}

impl Default for ProverClient {
    fn default() -> Self {
        EmbeddedClientBuilder::new(EmbeddedOptions::default())
            .build()
            .expect("Failed to initialize default ProverClient")
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
        }
    }

    fn run_setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.run_setup(program, with_hints),
        }
    }

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        opts: ProofOpts,
    ) -> Result<Proof> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.run_prove(program, input, executor, mode, opts),
        }
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
    ) -> Result<ExecuteResult> {
        match self.inner.as_ref() {
            BackendClient::Embedded(c) => c.run_execute(program, input, executor),
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
        }
    }
}
