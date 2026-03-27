use std::time::Duration;

use anyhow::Result;

use super::proof::Proof;
use crate::hints::ZiskHints;
use crate::GuestProgram;
use crate::ZiskStdin;
use crate::{Client, ExecutorKind, ProofMode};

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
    stdin: ZiskStdin,
    executor: Option<ExecutorKind>,
    hints: Option<ZiskHints>,
    timeout: Option<Duration>,
    proof_opts: Option<ProofOpts>,
    proof_kind: ProofKind,
    minimal_memory: bool,
    subscribers: Vec<(WatchEvent, Box<dyn Fn(WatchEvent) + Send + Sync>)>,
}

impl<'a, C: Client> ProveRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram, stdin: ZiskStdin) -> Self {
        Self {
            client,
            program,
            stdin,
            executor: None,
            hints: None,
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

    /// Set the hints source. Requires Assembly executor on the client builder.
    #[must_use]
    pub fn hints(mut self, hints: ZiskHints) -> Self {
        self.hints = Some(hints);
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
    // TODO: minimal_memory is stored but not forwarded to run_prove yet.
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
        self.client.run_prove(self.program, self.stdin, executor, self.hints, mode, opts)
    }
}

// pub struct ProofHandle { ... }
// pub fn submit(self) -> Result<ProofHandle> { todo!() }

// impl ProofHandle {
//     pub(crate) fn new(receiver: tokio::sync::oneshot::Receiver<Result<Proof>>) -> Self {
//         Self { receiver }
//     }
//
//     /// Await proof completion.
//     pub async fn proof(self) -> Result<Proof> {
//         self.receiver
//             .await
//             .map_err(|_| anyhow::anyhow!("prove task dropped before completing"))?
//     }
//
//     /// Register an event callback. Can be called after submission (post-submit).
//     pub fn on(&self, _event: WatchEvent, _cb: impl Fn(WatchEvent) + Send + Sync + 'static) {
//         todo!()
//     }
//
//     /// Cancel the in-flight proof job.
//     pub fn cancel(&self) -> Result<()> {
//         todo!()
//     }
// }
