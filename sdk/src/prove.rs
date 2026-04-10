use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use tokio::task::JoinHandle;

use crate::cancel::CancellationToken;
use crate::input::ProgramInput;
use crate::proof::Proof;
use crate::{Client, ExecutorKind, GuestProgram, ProofMode};

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

type Subscriber = (WatchEvent, Arc<dyn Fn(WatchEvent) + Send + Sync>);
type SubscriberList = Arc<Mutex<Vec<Subscriber>>>;

fn fire_event(subscribers: &SubscriberList, event: WatchEvent) {
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
#[allow(clippy::too_many_arguments)]
fn spawn_prove(
    client: impl Client + 'static,
    program: GuestProgram,
    input: ProgramInput,
    executor: ExecutorKind,
    mode: ProofMode,
    subscribers: SubscriberList,
    timeout: Option<Duration>,
    cancel_token: Option<CancellationToken>,
) -> ProofHandle {
    let subs = Arc::clone(&subscribers);
    let token_for_task = cancel_token.clone();
    let handle = tokio::task::spawn_blocking(move || {
        fire_event(&subs, WatchEvent::Started);
        let result = client.run_prove(&program, input, executor, mode, token_for_task.as_ref());
        match &result {
            Ok(_) => fire_event(&subs, WatchEvent::Completed),
            Err(e) => fire_event(&subs, WatchEvent::Failed(e.to_string())),
        }
        result
    });
    ProofHandle { handle, subscribers, timeout, cancel_token }
}

/// Builder for a prove request.
///
/// Obtain via [`crate::ProverClient::prove`].
/// Finalize with `.run()` (blocking) or `.submit()` (non-blocking, returns [`ProofHandle`]).
#[allow(dead_code)]
pub struct ProveRequest<C> {
    client: C,
    program: GuestProgram,
    input: ProgramInput,
    executor: Option<ExecutorKind>,
    timeout: Option<Duration>,
    proof_kind: ProofKind,
    subscribers: Vec<Subscriber>,
}

#[allow(private_bounds)]
impl<C: Client + Clone + Send + Sync + 'static> ProveRequest<C> {
    pub(crate) fn new(client: C, program: GuestProgram, input: impl Into<ProgramInput>) -> Self {
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
    ///
    /// [`ExecutorKind::Assembly`] requires it to be declared on the client builder;
    /// otherwise `.run()` / `.submit()` returns an error.
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

    /// Set the proof wrapping mode (alias for `proof_kind`).
    #[must_use]
    pub fn wrap_proof(mut self, kind: ProofKind) -> Self {
        self.proof_kind = kind;
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
            ProofKind::StarkMinimal => ProofMode::VadcopFinalMinimal,
            ProofKind::Plonk => ProofMode::Plonk,
        }
    }

    /// Sync: block the calling thread until the proof is ready.
    pub fn run(self) -> Result<Proof> {
        let mode = self.resolve_mode();
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        let client = self.client;
        let program = self.program;
        let input = self.input;
        let subscribers: SubscriberList = Arc::new(Mutex::new(self.subscribers));

        fire_event(&subscribers, WatchEvent::Started);

        let result = if let Some(dur) = self.timeout {
            let cancel_token = CancellationToken::new();
            let cancel_token2 = cancel_token.clone();
            let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
            let timed_out = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let timed_out2 = Arc::clone(&timed_out);
            let r = std::thread::scope(|s| {
                s.spawn(move || {
                    if stop_rx.recv_timeout(dur).is_err() {
                        timed_out2.store(true, std::sync::atomic::Ordering::Relaxed);
                        cancel_token2.cancel();
                    }
                });
                let r = client.run_prove(&program, input, executor, mode, Some(&cancel_token));
                let _ = stop_tx.send(());
                r
            });
            if timed_out.load(std::sync::atomic::Ordering::Acquire) {
                Err(anyhow::anyhow!("proof timed out after {dur:?}"))
            } else {
                r
            }
        } else {
            client.run_prove(&program, input, executor, mode, None)
        };

        match &result {
            Ok(_) => fire_event(&subscribers, WatchEvent::Completed),
            Err(e) => fire_event(&subscribers, WatchEvent::Failed(e.to_string())),
        }
        result
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
    pub fn run_async(self) -> Result<ProofHandle> {
        let mode = self.resolve_mode();
        let cancel_token =
            if self.timeout.is_some() { Some(CancellationToken::new()) } else { None };
        let subscribers: SubscriberList = Arc::new(Mutex::new(self.subscribers));
        Ok(spawn_prove(
            self.client,
            self.program,
            self.input,
            self.executor.unwrap_or(ExecutorKind::Emulator),
            mode,
            subscribers,
            self.timeout,
            cancel_token,
        ))
    }
}

/// Handle to an in-flight async proof generation task.
///
/// Obtained by calling `.submit()` on a [`ProveRequest`].
pub struct ProofHandle {
    handle: JoinHandle<Result<Proof>>,
    subscribers: SubscriberList,
    timeout: Option<Duration>,
    cancel_token: Option<CancellationToken>,
}

impl ProofHandle {
    /// Await proof completion and return the proof.
    ///
    /// Returns an error if the task panicked or was cancelled via [`cancel`](Self::cancel).
    /// If a timeout was set via `.timeout()`, fires [`WatchEvent::Failed`] and returns an error
    /// once the deadline is exceeded.
    pub async fn proof(self) -> Result<Proof> {
        let mut handle = self.handle;
        match (self.timeout, self.cancel_token) {
            (Some(dur), Some(cancel_token)) => match tokio::time::timeout(dur, &mut handle).await {
                Ok(join_result) => {
                    join_result.map_err(|e| anyhow::anyhow!("Proof task failed: {e}"))?
                }
                Err(_elapsed) => {
                    cancel_token.cancel();
                    fire_event(
                        &self.subscribers,
                        WatchEvent::Failed(format!("Proof timed out after {dur:?}")),
                    );
                    anyhow::bail!("Proof timed out after {dur:?}")
                }
            },
            _ => handle.await.map_err(|e| anyhow::anyhow!("Proof task failed: {e}"))?,
        }
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
    /// Signals the cancellation token (allowing cooperative cancellation at checkpoints)
    /// and aborts the Tokio task handle.
    pub fn cancel(self) {
        if let Some(token) = &self.cancel_token {
            token.cancel();
        }
        self.handle.abort();
    }
}
