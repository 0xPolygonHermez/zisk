use std::time::Duration;

use anyhow::Result;

use crate::GuestProgram;
use crate::{Client, ExecutorKind};
use zisk_common::io::ZiskStdin;
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

    pub fn get_duration(&self) -> Duration {
        self.inner.get_duration()
    }
}

/// Builder for a dry-run execution request (no proof).
///
/// Obtain via `client.execute(&program, stdin)`.
#[allow(dead_code)]
pub struct ExecuteRequest<'a, C: Client> {
    client: &'a C,
    program: &'a GuestProgram,
    stdin: ZiskStdin,
    executor: Option<ExecutorKind>,
    timeout: Option<Duration>,
    traces: Vec<Tracing>,
}

impl<'a, C: Client> ExecuteRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram, stdin: ZiskStdin) -> Self {
        Self { client, program, stdin, executor: None, timeout: None, traces: Vec::new() }
    }

    /// Override the executor for this execute call.
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Set a timeout for the execution.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Enable a tracing mode.
    #[must_use]
    pub fn trace(mut self, tracing: Tracing) -> Self {
        self.traces.push(tracing);
        self
    }

    /// Run the execution synchronously.
    pub fn run(self) -> Result<ExecuteResult> {
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        self.client.run_execute(self.program, self.stdin, executor)
    }
}
