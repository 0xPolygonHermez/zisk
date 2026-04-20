use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use zisk_common::ProofKind;
use zisk_prover_backend::{GuestProgram, ProveOutput};

use crate::input::ProgramInput;
use crate::job_handle::{JobHandle, JobId, Subscriber, SubscriberList};
use crate::{Client, ExecutorKind};

pub struct ProveResult {
    pub(crate) job_id: Option<JobId>,
    output: ProveOutput,
}

impl ProveResult {
    pub fn new(output: ProveOutput, job_id: Option<JobId>) -> Self {
        Self { output, job_id }
    }

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
/// Obtain via [`crate::ProverClient::prove`].
/// Finalize with `.run()` which returns a [`JobHandle<ProveResult>`].
pub struct ProveRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
    input: ProgramInput,
    executor: Option<ExecutorKind>,
    timeout: Option<Duration>,
    proof_kind: ProofKind,
    subscribers: Vec<Subscriber>,
}

#[allow(private_bounds)]
impl<'a, C: Client> ProveRequest<'a, C> {
    pub(crate) fn new(
        client: &'a C,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> Self {
        Self {
            client,
            program,
            input: input.into(),
            executor: None,
            timeout: None,
            proof_kind: ProofKind::default(),
            subscribers: Vec::new(),
        }
    }

    /// Override the executor for this prove call.
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = Some(executor);
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

    fn resolve_mode(&self) -> ProofKind {
        self.proof_kind
    }

    /// Submit proof generation, returning a [`JobHandle<ProveResult>`] immediately.
    pub fn run(self) -> Result<JobHandle<ProveResult>> {
        let mode = self.resolve_mode();
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        let subs: SubscriberList = Arc::new(Mutex::new(self.subscribers));
        self.client.run_prove(self.program, self.input, executor, mode, self.timeout, subs)
    }
}
