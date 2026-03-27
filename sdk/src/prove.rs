use std::time::Duration;

use anyhow::Result;

use super::proof::Proof;
use crate::async_prove::{spawn_prove, Subscriber, SubscriberList};
use crate::input::ProgramInput;
use crate::GuestProgram;
use crate::{Client, ExecutorKind, ProofHandle, ProofMode};
use std::sync::{Arc, Mutex};

/// Events emitted during proof generation.
///
/// `WatchEvent::All` is a subscription filter meaning "receive all events".
/// It is never emitted as a concrete event in callbacks.
#[derive(Debug, Clone, PartialEq)]
pub enum WatchEvent {
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
use zisk_prover_backend::ProofOpts;

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
/// Obtain via `client.prove(&program, stdin)`.
/// Finalize with `.run()` (sync).
#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub struct ProveRequest<'a, C: Client> {
    client: &'a C,
    program: &'a GuestProgram,
    input: ProgramInput,
    executor: Option<ExecutorKind>,
    timeout: Option<Duration>,
    proof_opts: Option<ProofOpts>,
    proof_kind: ProofKind,
    minimal_memory: bool,
    subscribers: Vec<(WatchEvent, Box<dyn Fn(WatchEvent) + Send + Sync>)>,
}

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
            proof_opts: None,
            proof_kind: ProofKind::default(),
            minimal_memory: false,
            subscribers: Vec::new(),
        }
    }

    /// Override the executor for this prove call.
    ///
    /// `Executor::Assembly` requires it to be declared on the client builder;
    /// otherwise `.run()` returns an error.
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Set proof generation options (e.g. minimal memory mode).
    #[must_use]
    pub fn with_proof_options(mut self, opts: ProofOpts) -> Self {
        self.proof_opts = Some(opts);
        self
    }

    /// Set a timeout for proof generation.
    // TODO: timeout is stored but not enforced in run() yet.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Use minimal memory mode during execution.
    #[must_use]
    pub fn minimal_memory(mut self) -> Self {
        self.minimal_memory = true;
        self
    }

    /// Generate a full STARK proof (default).
    #[must_use]
    pub fn stark(mut self) -> Self {
        self.proof_kind = ProofKind::Stark;
        self
    }

    /// Generate a STARK proof in minimal-memory mode.
    #[must_use]
    pub fn stark_minimal(mut self) -> Self {
        self.proof_kind = ProofKind::StarkMinimal;
        self
    }

    /// Generate a PLONK/SNARK proof.
    #[must_use]
    pub fn plonk(mut self) -> Self {
        self.proof_kind = ProofKind::Plonk;
        self
    }

    /// Register an event callback. Can be called before submission (pre-submit).
    // TODO: subscribers are stored but never invoked in run() yet.
    #[must_use]
    pub fn on(
        mut self,
        event: WatchEvent,
        cb: impl Fn(WatchEvent) + Send + Sync + 'static,
    ) -> Self {
        self.subscribers.push((event, Box::new(cb)));
        self
    }

    /// Sync: blocks the calling thread until the proof is ready.
    pub fn run(self) -> Result<Proof> {
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        // TODO: enforce self.timeout — abort/cancel the blocking call on deadline
        // TODO: fire self.subscribers (Started, Progress, Completed, Failed) during execution
        let mode = match self.proof_kind {
            ProofKind::Stark => ProofMode::VadcopFinal,
            ProofKind::StarkMinimal => ProofMode::VadcopFinalReduced,
            ProofKind::Plonk => ProofMode::Snark,
        };
        let mut opts = self.proof_opts.unwrap_or_default();
        if self.minimal_memory {
            opts = opts.minimal_memory();
        }
        self.client.run_prove(self.program, self.input, executor, mode, opts)
    }

    /// Async: submit proof generation to a background thread, returning a [`crate::ProofHandle`] immediately.
    ///
    /// Requires `C: Clone + Send + Sync + 'static` and an active Tokio runtime.
    /// See [`crate::ProofHandle`] for awaiting completion, post-submit callbacks, and cancellation.
    pub fn submit(self) -> Result<ProofHandle>
    where
        C: Clone + 'static,
    {
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        let mode = match self.proof_kind {
            ProofKind::Stark => ProofMode::VadcopFinal,
            ProofKind::StarkMinimal => ProofMode::VadcopFinalReduced,
            ProofKind::Plonk => ProofMode::Snark,
        };
        let mut opts = self.proof_opts.unwrap_or_default();
        if self.minimal_memory {
            opts = opts.minimal_memory();
        }
        let subscribers: SubscriberList = Arc::new(Mutex::new(
            self.subscribers
                .into_iter()
                .map(|(e, b)| -> Subscriber { (e, Arc::from(b)) })
                .collect(),
        ));
        Ok(spawn_prove(
            self.client.clone(),
            Arc::new(self.program.clone()),
            self.input,
            executor,
            mode,
            opts,
            subscribers,
        ))
    }
}
