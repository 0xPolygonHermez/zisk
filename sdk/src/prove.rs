use std::time::Duration;

use anyhow::Result;

use super::client::ProverClient;
use super::proof::Proof;
use super::types::{ClientConfig, Executor, WatchEvent};
use crate::hints::ZiskHints;
use crate::GuestProgram;
use zisk_common::io::ZiskStdin;
use zisk_prover_backend::ProofOpts;

/// Builder for a prove request.
///
/// Obtain via `client.prove(&program, stdin)`.
/// Finalize with `.run()` (sync).
#[allow(dead_code)]
pub struct ProveRequest<'a> {
    client: &'a ProverClient,
    program: &'a GuestProgram,
    stdin: ZiskStdin,
    executor: Option<Executor>,
    hints: Option<ZiskHints>,
    timeout: Option<Duration>,
    proof_opts: Option<ProofOpts>,
    minimal_memory: bool,
    subscribers: Vec<(WatchEvent, Box<dyn Fn(WatchEvent) + Send + Sync>)>,
}

impl<'a> ProveRequest<'a> {
    pub(crate) fn new(
        client: &'a ProverClient,
        program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> Self {
        Self {
            client,
            program,
            stdin,
            executor: None,
            hints: None,
            timeout: None,
            proof_opts: None,
            minimal_memory: false,
            subscribers: Vec::new(),
        }
    }

    /// Override the executor for this prove call.
    ///
    /// `Executor::Assembly` requires it to be declared on the client builder;
    /// otherwise `.run()` returns an error.
    #[must_use]
    pub fn executor(mut self, executor: Executor) -> Self {
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

    /// Select STARK proof type (default).
    #[must_use]
    pub fn stark(self) -> Self {
        self
    }

    /// Register an event callback. Can be called before submission (pre-submit).
    #[must_use]
    pub fn on(
        mut self,
        event: WatchEvent,
        cb: impl Fn(WatchEvent) + Send + Sync + 'static,
    ) -> Self {
        self.subscribers.push((event, Box::new(cb)));
        self
    }

    fn validate(&self) -> Result<()> {
        let executor = self.executor.unwrap_or_else(|| self.client.default_executor());
        if executor == Executor::Assembly && !self.client.assembly_enabled() {
            anyhow::bail!(
                "Assembly executor not enabled — call .executor(Executor::Assembly) on the builder"
            );
        }
        if self.hints.is_some() && !self.client.assembly_enabled() {
            anyhow::bail!(
                "Hints require Assembly executor — call .executor(Executor::Assembly) on the builder"
            );
        }
        Ok(())
    }

    /// Sync: blocks the calling thread until the proof is ready.
    pub fn run(self) -> Result<Proof> {
        self.validate()?;
        self.client.run_prove(self.program, self.stdin, self.proof_opts.unwrap_or_default())
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
