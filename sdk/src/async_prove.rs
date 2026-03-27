use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use tokio::task::JoinHandle;
use zisk_prover_backend::ProofOpts;

use crate::input::ProgramInput;
use crate::proof::Proof;
use crate::prove::{ProofKind, WatchEvent};
use crate::{Client, ExecutorKind, GuestProgram, ProofMode};

pub(crate) type Subscriber = (WatchEvent, Arc<dyn Fn(WatchEvent) + Send + Sync>);
pub(crate) type SubscriberList = Arc<Mutex<Vec<Subscriber>>>;

pub(crate) fn fire_event(subscribers: &SubscriberList, event: WatchEvent) {
    // Snapshot matching callbacks before releasing the lock to avoid re-entrancy issues
    // (e.g. a callback that calls handle.on() would deadlock if we held the lock).
    let matching: Vec<Arc<dyn Fn(WatchEvent) + Send + Sync>> = match subscribers.lock() {
        Ok(subs) => subs
            .iter()
            .filter(|(filter, _)| *filter == WatchEvent::All || *filter == event)
            .map(|(_, cb)| Arc::clone(cb))
            .collect(),
        Err(_) => return,
    };
    for cb in matching {
        cb(event.clone());
    }
}

/// Spawn a blocking proof task and return a [`ProofHandle`].
///
/// Shared by [`AsyncProveRequest::submit`] and [`crate::prove::ProveRequest::submit`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_prove(
    client: impl Client + 'static,
    program: Arc<GuestProgram>,
    input: ProgramInput,
    executor: ExecutorKind,
    mode: ProofMode,
    opts: ProofOpts,
    subscribers: SubscriberList,
) -> ProofHandle {
    let subs = Arc::clone(&subscribers);
    let handle = tokio::task::spawn_blocking(move || {
        fire_event(&subs, WatchEvent::Started);
        let result = client.run_prove(&program, input, executor, mode, opts);
        match &result {
            Ok(_) => fire_event(&subs, WatchEvent::Completed),
            Err(e) => fire_event(&subs, WatchEvent::Failed(e.to_string())),
        }
        result
    });
    ProofHandle { handle, subscribers }
}

/// Async builder for a prove request.
///
/// Obtain via [`ProverClient::prove_async`].
/// Finalize with `.submit()` (non-blocking, returns [`ProofHandle`]) or `.run()` (blocking).
#[allow(dead_code)]
pub struct AsyncProveRequest<C: Client + Clone + Send + Sync + 'static> {
    client: C,
    program: Arc<GuestProgram>,
    input: ProgramInput,
    executor: Option<ExecutorKind>,
    timeout: Option<Duration>,
    proof_opts: Option<ProofOpts>,
    proof_kind: ProofKind,
    minimal_memory: bool,
    subscribers: Vec<Subscriber>,
}

impl<C: Client + Clone + Send + Sync + 'static> AsyncProveRequest<C> {
    pub(crate) fn new(
        client: C,
        program: Arc<GuestProgram>,
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
    /// [`ExecutorKind::Assembly`] requires it to be declared on the client builder;
    /// otherwise `.run()` / `.submit()` returns an error.
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Set proof generation options.
    #[must_use]
    pub fn with_proof_options(mut self, opts: ProofOpts) -> Self {
        self.proof_opts = Some(opts);
        self
    }

    /// Set a timeout for proof generation.
    // TODO: timeout is stored but not yet enforced.
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

    /// Generate a reduced STARK proof.
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

    /// Register a pre-submit event callback.
    ///
    /// Use [`WatchEvent::All`] to subscribe to all events.
    /// Callbacks registered here are guaranteed to receive [`WatchEvent::Started`].
    #[must_use]
    pub fn on(
        mut self,
        event: WatchEvent,
        cb: impl Fn(WatchEvent) + Send + Sync + 'static,
    ) -> Self {
        self.subscribers.push((event, Arc::new(cb)));
        self
    }

    fn resolve_mode(&self) -> ProofMode {
        match self.proof_kind {
            ProofKind::Stark => ProofMode::VadcopFinal,
            ProofKind::StarkMinimal => ProofMode::VadcopFinalReduced,
            ProofKind::Plonk => ProofMode::Snark,
        }
    }

    fn resolve_opts(&self) -> ProofOpts {
        let mut opts = self.proof_opts.clone().unwrap_or_default();
        if self.minimal_memory {
            opts = opts.minimal_memory();
        }
        opts
    }

    /// Sync: block the calling thread until the proof is ready.
    pub fn run(self) -> Result<Proof> {
        let mode = self.resolve_mode();
        let opts = self.resolve_opts();
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        // self.timeout and self.subscribers dropped here
        // TODO: enforce timeout; fire subscribers
        self.client.run_prove(&self.program, self.input, executor, mode, opts)
    }

    /// Async: submit proof generation to a background thread, returning a [`ProofHandle`] immediately.
    ///
    /// Fires [`WatchEvent::Started`] before proving begins, then
    /// [`WatchEvent::Completed`] or [`WatchEvent::Failed`] on completion.
    ///
    /// Pre-submit callbacks (`.on()`) are guaranteed to receive all events.
    /// Post-submit callbacks added via [`ProofHandle::on`] may miss [`WatchEvent::Started`].
    ///
    /// Requires an active Tokio runtime.
    pub fn submit(self) -> Result<ProofHandle> {
        let mode = self.resolve_mode();
        let opts = self.resolve_opts();
        let subscribers: SubscriberList = Arc::new(Mutex::new(self.subscribers));
        Ok(spawn_prove(
            self.client,
            self.program,
            self.input,
            self.executor.unwrap_or(ExecutorKind::Emulator),
            mode,
            opts,
            subscribers,
        ))
    }
}

/// Handle to an in-flight async proof generation task.
///
/// Obtained by calling `.submit()` on an [`AsyncProveRequest`].
pub struct ProofHandle {
    handle: JoinHandle<Result<Proof>>,
    subscribers: SubscriberList,
}

impl ProofHandle {
    /// Await proof completion and return the proof.
    ///
    /// Returns an error if the task panicked or was cancelled via [`cancel`](Self::cancel).
    pub async fn proof(self) -> Result<Proof> {
        self.handle.await.map_err(|e| anyhow::anyhow!("Proof task failed: {e}"))?
    }

    /// Register a post-submit event callback.
    ///
    /// Use [`WatchEvent::All`] to subscribe to all events.
    /// Note: may miss [`WatchEvent::Started`] if the task has already begun.
    pub fn on(&self, event: WatchEvent, cb: impl Fn(WatchEvent) + Send + Sync + 'static) {
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push((event, Arc::new(cb)));
        }
    }

    /// Cancel the proof task.
    ///
    /// The spawned blocking OS thread continues running to completion, but
    /// [`proof()`](Self::proof) returns an error immediately.
    pub fn cancel(self) {
        self.handle.abort();
    }
}
