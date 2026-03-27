use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use zisk_common::{io::ZiskStdin, ZiskProgramVK};
use zisk_prover_backend::{GuestProgram, ProofOpts};

use crate::{
    execute::{ExecuteRequest, ExecuteResult},
    proof::Proof,
    prove::ProveRequest,
    prover::{EmbeddedClient, EmbeddedClientBuilder, EmbeddedOptions},
    setup::SetupRequest,
    upload::UploadRequest,
    Client, ExecutorKind,
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
    inner: BackendClient,
}

impl ProverClient {
    pub(crate) fn from_embedded(client: EmbeddedClient) -> Self {
        ensure_single_instance();
        Self { inner: BackendClient::Embedded(client) }
    }

    pub fn embedded(options: EmbeddedOptions) -> EmbeddedClientBuilder {
        EmbeddedClientBuilder::new(options)
    }

    // pub fn remote(url: impl Into<String>) -> RemoteClientBuilder {
    //     ensure_single_instance();
    //     RemoteClientBuilder::new(url.into())
    // }

    pub fn vk(&self, program: &GuestProgram) -> Result<ZiskProgramVK> {
        match &self.inner {
            BackendClient::Embedded(c) => c.vk(program),
        }
    }

    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, stdin)
    }

    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> ExecuteRequest<'a, Self> {
        ExecuteRequest::new(self, program, stdin)
    }

    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    pub fn upload<'a>(&'a self, program: &'a GuestProgram) -> UploadRequest<'a, Self> {
        UploadRequest::new(self, program)
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
        PROVER_CLIENT_CREATED.store(false, Ordering::Release);
    }
}

impl Client for ProverClient {
    fn run_prove(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        executor: ExecutorKind,
        opts: ProofOpts,
    ) -> Result<Proof> {
        match &self.inner {
            BackendClient::Embedded(c) => c.run_prove(program, stdin, executor, opts),
        }
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        executor: ExecutorKind,
    ) -> Result<ExecuteResult> {
        match &self.inner {
            BackendClient::Embedded(c) => c.run_execute(program, stdin, executor),
        }
    }
}
