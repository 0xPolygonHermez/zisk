use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use crate::Result;
use zisk_common::ProofKind;
use zisk_prover_backend::{GuestProgram, ProveOutput};

use crate::hints::HintsSource;
use crate::input_source::InputSource;
use crate::job_handle::{subscriber_list_from, JobHandle, JobId, Subscriber, SubscriberList};
use crate::{Client, ClientSync, ExecutorKind};

/// Result of a prove operation.
pub struct ProveResult {
    pub(crate) job_id: Option<JobId>,
    output: ProveOutput,
}

impl ProveResult {
    /// Create a new `ProveResult` with the given output and job ID.
    pub fn new(output: ProveOutput, job_id: Option<JobId>) -> Self {
        Self { output, job_id }
    }

    /// Get the ID of the job that produced this result, if available.
    pub fn job_id(&self) -> Option<&JobId> {
        self.job_id.as_ref()
    }
}

impl Deref for ProveResult {
    type Target = ProveOutput;
    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl From<ProveOutput> for ProveResult {
    fn from(output: ProveOutput) -> Self {
        Self { output, job_id: None }
    }
}

/// Events emitted during proof generation.
///
/// `JobEvent::All` is a subscription filter meaning "receive all events".
/// It is never emitted as a concrete event in callbacks.
#[derive(Debug, Clone, PartialEq)]
pub enum JobEvent {
    /// Subscribe to all events (filter only; never emitted to callbacks).
    All,
    /// Job accepted and execution started.
    Started,
    /// Proof generation progress (0–100).
    Progress(u8),
    /// Proof completed successfully.
    Completed,
    /// Proof generation failed.
    Failed(String),
}

/// Builder for a prove request.
///
/// Obtain via [`EmbeddedClient::prove`](crate::EmbeddedClient::prove),
/// [`RemoteClient::prove`](crate::RemoteClient::prove), or
/// [`ZiskClient::prove`](crate::ZiskClient::prove).
/// Finalize with `.run()` which returns a [`JobHandle<ProveResult>`].
pub struct ProveRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
    stdin: InputSource,
    hints: Option<HintsSource>,
    executor: ExecutorKind,
    timeout: Option<Duration>,
    proof_kind: ProofKind,
    subscribers: Vec<Subscriber>,
}

#[allow(private_bounds)]
impl<'a, C: Client> ProveRequest<'a, C> {
    pub(crate) fn new(
        client: &'a C,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
        executor: ExecutorKind,
    ) -> Self {
        Self {
            client,
            program,
            stdin: stdin.into(),
            hints: None,
            executor,
            timeout: None,
            proof_kind: ProofKind::default(),
            subscribers: Vec::new(),
        }
    }

    /// Attach a hints stream to this prove request.
    ///
    /// Requires the program to have been set up with
    /// [`SetupRequest::with_hints`](crate::SetupRequest::with_hints) and
    /// the [`ExecutorKind::Assembly`] executor.
    #[must_use]
    pub fn hints(mut self, hints: impl Into<HintsSource>) -> Self {
        self.hints = Some(hints.into());
        self
    }

    /// Override the executor for this prove call.
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = executor;
        self
    }

    /// Set a timeout for proof generation.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Set the proof wrapping mode.
    #[must_use]
    pub fn wrap(mut self, kind: ProofKind) -> Self {
        self.proof_kind = kind;
        self
    }

    /// Register a pre-submit event callback.
    ///
    /// Use [`JobEvent::All`] to subscribe to all events.
    #[must_use]
    pub fn on(mut self, event: JobEvent, cb: impl Fn(JobEvent) + Send + Sync + 'static) -> Self {
        self.subscribers.push((event, Arc::new(cb)));
        self
    }

    /// Submit proof generation, returning a [`JobHandle<ProveResult>`] immediately.
    pub fn run(self) -> Result<JobHandle<ProveResult>> {
        let subs: SubscriberList = subscriber_list_from(self.subscribers);
        self.client.run_prove(
            self.program,
            self.stdin,
            self.hints,
            self.executor,
            self.proof_kind,
            self.timeout,
            subs,
        )
    }
}

#[allow(private_bounds)]
impl<'a, C: ClientSync> ProveRequest<'a, C> {
    /// Run proof generation synchronously, returning the result directly.
    ///
    /// Unlike [`run`](Self::run), this drives the work on the calling thread and
    /// requires no async runtime — use it when embedding the SDK in a
    /// synchronous program. Registered [`on`](Self::on) callbacks fire
    /// synchronously during the call. Available only for the embedded client
    /// ([`EmbeddedClient`](crate::EmbeddedClient)).
    pub fn run_sync(self) -> Result<ProveResult> {
        let subs = subscriber_list_from(self.subscribers);
        self.client.run_prove_sync(
            self.program,
            self.stdin,
            self.hints,
            self.executor,
            self.proof_kind,
            subs,
        )
    }
}
