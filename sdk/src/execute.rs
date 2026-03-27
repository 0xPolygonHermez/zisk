use std::time::Duration;

use anyhow::Result;

use crate::input::ProgramInput;
use crate::GuestProgram;
use crate::{Client, ExecutorKind};
use std::sync::Arc;
use zisk_common::StatsCostPerType;
use zisk_prover_backend::ZiskExecuteResult;

/// Tracing options for program execution.
#[derive(Debug, Clone)]
pub enum Tracing {
    /// Trace the input data.
    Input,
    /// Trace the hints stream.
    Hints,
    /// Print an execution summary.
    Summary,
}

/// Result of a dry-run program execution (no proof generated).
pub struct ExecuteResult {
    pub(crate) inner: ZiskExecuteResult,
}

impl ExecuteResult {
    pub(crate) fn new(inner: ZiskExecuteResult) -> Self {
        Self { inner }
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.inner.get_execution_steps()
    }

    pub fn get_execution_total_cost(&self) -> u64 {
        self.inner.get_execution_total_cost()
    }

    pub fn get_execution_cost_per_type(&self) -> &StatsCostPerType {
        self.inner.get_execution_cost_per_type()
    }

    pub fn get_duration(&self) -> Duration {
        self.inner.get_duration()
    }

    pub fn get_public_values<T: serde::de::DeserializeOwned + serde::Serialize>(
        &self,
    ) -> Result<T> {
        self.inner.get_public_values()
    }
}

/// Builder for a dry-run execution request (no proof).
///
/// Obtain via `client.execute(&program, stdin)`.
#[allow(dead_code)]
pub struct ExecuteRequest<'a, C: Client> {
    client: &'a C,
    program: &'a GuestProgram,
    input: ProgramInput,
    executor: Option<ExecutorKind>,
    timeout: Option<Duration>,
    traces: Vec<Tracing>,
    cancel_fn: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl<'a, C: Client> ExecuteRequest<'a, C> {
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
            traces: Vec::new(),
            cancel_fn: None,
        }
    }

    /// Override the executor for this execute call.
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Set a timeout for the execution.
    // TODO: timeout is stored but not enforced in run() yet.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Enable a tracing mode.
    // TODO: traces are stored but not forwarded to run_execute yet.
    #[must_use]
    pub fn trace(mut self, tracing: Tracing) -> Self {
        self.traces.push(tracing);
        self
    }

    pub(crate) fn with_cancel_fn(mut self, f: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.cancel_fn = Some(f);
        self
    }

    /// Run the execution synchronously.
    pub fn run(self) -> Result<ExecuteResult> {
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        let client = self.client;
        let program = self.program;
        let input = self.input;
        let cancel_fn = self.cancel_fn;

        if let Some(dur) = self.timeout {
            let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
            let timed_out = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let timed_out2 = Arc::clone(&timed_out);
            let result = std::thread::scope(|s| {
                s.spawn(move || {
                    if stop_rx.recv_timeout(dur).is_err() {
                        timed_out2.store(true, std::sync::atomic::Ordering::Relaxed);
                        if let Some(ref f) = cancel_fn {
                            f();
                        }
                    }
                });
                let result = client.run_execute(program, input, executor);
                let _ = stop_tx.send(());
                result
            });
            if timed_out.load(std::sync::atomic::Ordering::Acquire) {
                anyhow::bail!("execution timed out after {dur:?}");
            }
            result
        } else {
            client.run_execute(program, input, executor)
        }
    }
}
