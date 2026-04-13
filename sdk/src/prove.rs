use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use crate::input::ProgramInput;
use crate::job_handle::{JobHandle, Subscriber, SubscriberList};
use crate::proof::Proof;
use crate::{Client, ExecutorKind, ProofMode};

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

/// The kind of proof to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProofKind {
    /// Full STARK proof (default).
    #[default]
    Stark,
    /// STARK proof in minimal-memory mode.
    StarkMinimal,
    /// PLONK/SNARK proof (requires a prior STARK reduction).
    Plonk,
}

/// Builder for a prove request.
///
/// Obtain via [`crate::ProverClient::prove`].
/// Finalize with `.run()` which returns a [`JobHandle<Proof>`].
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

    fn resolve_mode(&self) -> ProofMode {
        match self.proof_kind {
            ProofKind::Stark => ProofMode::VadcopFinal,
            ProofKind::StarkMinimal => ProofMode::VadcopFinalMinimal,
            ProofKind::Plonk => ProofMode::Plonk,
        }
    }

    /// Submit proof generation, returning a [`JobHandle<Proof>`] immediately.
    pub fn run(self) -> Result<JobHandle<Proof>> {
        let mode = self.resolve_mode();
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        let subs: SubscriberList = Arc::new(Mutex::new(self.subscribers));
        self.client.run_prove(self.program, self.input, executor, mode, self.timeout, subs)
    }
}
