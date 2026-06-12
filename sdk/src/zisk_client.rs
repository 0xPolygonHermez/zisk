//! Backend-agnostic client — the default entry point for proving.
//!
//! [`ZiskClient`] holds either an [`EmbeddedClient`] or a [`RemoteClient`] and dispatches at
//! runtime. Reach for it first: it covers the common operations
//! (`upload`/`setup`/`prove`/`execute`/`wrap_proof`) and lets you pick the backend at runtime
//! (e.g. from a CLI flag) with a single binding for both paths:
//!
//! ```rust,ignore
//! use zisk_sdk::{ZiskClient, ExecutorKind};
//!
//! # async fn example(embedded: bool, elf: &zisk_sdk::GuestProgram) -> zisk_sdk::Result<()> {
//! let client = if embedded {
//!     ZiskClient::embedded().executor(ExecutorKind::Assembly).build()?
//! } else {
//!     ZiskClient::remote("http://127.0.0.1:7000").build()?
//! };
//!
//! client.upload(elf).run()?;
//! client.setup(elf).run()?.await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Specialized capabilities live on the concrete clients
//!
//! Because the backend is not known at compile time, `ZiskClient` exposes only the
//! **async** `run()` path and the operations both backends share. It intentionally does
//! *not* implement [`ClientSync`](crate::ClientSync) (`.run_sync()`) — a [`RemoteClient`]
//! performs network I/O and has no synchronous form — and backend-specific operations
//! (`verify_constraints`, embedded-only; `setup_by_id`, remote-only) are unavailable here.
//! This is by design: capabilities only one backend can honor are guarded at compile time
//! on the concrete type, never deferred to a runtime error.
//!
//! When you need one of those, recover the concrete client with [`ZiskClient::as_embedded`] /
//! [`ZiskClient::as_remote`]:
//!
//! ```rust,ignore
//! # use zisk_sdk::ZiskClient;
//! # fn example(client: &ZiskClient, elf: &zisk_sdk::GuestProgram, stdin: zisk_sdk::ZiskStdin) -> zisk_sdk::Result<()> {
//! if let Some(embedded) = client.as_embedded() {
//!     // sync path — no async runtime needed
//!     embedded.prove(elf, stdin).run_sync()?;
//! }
//! # Ok(())
//! # }
//! ```

use std::time::Duration;

use crate::Result;
use zisk_common::{ProgramVK, Proof, ProofKind, PublicValues};
use zisk_prover_backend::GuestProgram;

use crate::{
    execute::{ExecuteRequest, ExecuteResult},
    hints::HintsSource,
    input_source::InputSource,
    job_handle::{JobHandle, SubscriberList},
    prove::{ProveRequest, ProveResult},
    setup::{SetupRequest, SetupResult},
    upload::{UploadRequest, UploadResult},
    wrap::WrapRequest,
    Client, EmbeddedClient, ExecutorKind, RemoteClient,
};

#[derive(Clone)]
enum Inner {
    Embedded(EmbeddedClient),
    Remote(RemoteClient),
}

/// A client that wraps either an [`EmbeddedClient`] or a [`RemoteClient`], chosen at runtime.
///
/// Construct it via [`ZiskClient::embedded`] / [`ZiskClient::remote`] (whose `build()` yields an
/// `ZiskClient` directly), or from an already-built backend client via [`From`]/[`Into`]. See the
/// type-level documentation for the runtime-dispatch example and the operations that are
/// intentionally unavailable on this type.
#[derive(Clone)]
pub struct ZiskClient {
    inner: Inner,
    /// The executor forwarded into `prove`/`execute` requests. Captured from the embedded
    /// client's configuration; defaults for remote (which selects per-request).
    executor: ExecutorKind,
}

impl From<EmbeddedClient> for ZiskClient {
    fn from(client: EmbeddedClient) -> Self {
        let executor = client.executor();
        Self { inner: Inner::Embedded(client), executor }
    }
}

impl From<RemoteClient> for ZiskClient {
    fn from(client: RemoteClient) -> Self {
        Self { inner: Inner::Remote(client), executor: ExecutorKind::default() }
    }
}

impl Client for ZiskClient {
    fn run_upload(&self, program: &GuestProgram) -> Result<UploadResult> {
        match &self.inner {
            Inner::Embedded(c) => c.run_upload(program),
            Inner::Remote(c) => c.run_upload(program),
        }
    }

    fn run_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        match &self.inner {
            Inner::Embedded(c) => c.run_setup(program, with_hints, emulator_only, timeout, subs),
            Inner::Remote(c) => c.run_setup(program, with_hints, emulator_only, timeout, subs),
        }
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
    ) -> Result<JobHandle<ProveResult>> {
        match &self.inner {
            Inner::Embedded(c) => {
                c.run_prove(program, stdin, hints, executor, proof_kind, timeout, subs)
            }
            Inner::Remote(c) => {
                c.run_prove(program, stdin, hints, executor, proof_kind, timeout, subs)
            }
        }
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
        match &self.inner {
            Inner::Embedded(c) => c.run_execute(program, stdin, hints, executor, timeout, subs),
            Inner::Remote(c) => c.run_execute(program, stdin, hints, executor, timeout, subs),
        }
    }

    fn run_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<PublicValues>,
        override_program_vk: Option<ProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        match &self.inner {
            Inner::Embedded(c) => {
                c.run_wrap(proof, proof_kind, override_publics, override_program_vk, timeout, subs)
            }
            Inner::Remote(c) => {
                c.run_wrap(proof, proof_kind, override_publics, override_program_vk, timeout, subs)
            }
        }
    }
}

impl ZiskClient {
    /// Returns a builder for the embedded (local) backend whose `build()` yields an [`ZiskClient`].
    ///
    /// Use this when the backend is selected at runtime. For the concrete, fully-typed client
    /// (with the synchronous path and `verify_constraints`), use
    /// [`ProverClient::embedded`](crate::ProverClient::embedded) instead.
    #[must_use]
    pub fn embedded() -> crate::EmbeddedClientBuilder<Self> {
        crate::EmbeddedClientBuilder::for_output()
    }

    /// Returns a builder for the remote (coordinator) backend whose `build()` yields an [`ZiskClient`].
    ///
    /// Use this when the backend is selected at runtime. For the concrete, fully-typed client
    /// (with `setup_by_id`), use [`ProverClient::remote`](crate::ProverClient::remote) instead.
    #[must_use]
    pub fn remote(url: impl Into<String>) -> crate::RemoteClientBuilder<Self> {
        crate::RemoteClientBuilder::new(url)
    }

    /// Borrow the underlying [`EmbeddedClient`], or `None` if this client wraps a remote backend.
    ///
    /// Use this to reach embedded-only capabilities that `ZiskClient` cannot expose — the
    /// synchronous `.run_sync()` path and
    /// [`verify_constraints`](crate::VerifyConstraintsExtension::verify_constraints).
    #[must_use]
    pub fn as_embedded(&self) -> Option<&EmbeddedClient> {
        match &self.inner {
            Inner::Embedded(c) => Some(c),
            Inner::Remote(_) => None,
        }
    }

    /// Borrow the underlying [`RemoteClient`], or `None` if this client wraps an embedded backend.
    ///
    /// Use this to reach remote-only capabilities that `ZiskClient` cannot expose — e.g.
    /// [`setup_by_id`](crate::RemoteClient::setup_by_id).
    #[must_use]
    pub fn as_remote(&self) -> Option<&RemoteClient> {
        match &self.inner {
            Inner::Remote(c) => Some(c),
            Inner::Embedded(_) => None,
        }
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

    /// Submit an upload request.
    ///
    /// No-op for the embedded backend (the program is available locally); registers the
    /// ELF with the coordinator for the remote backend.
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
