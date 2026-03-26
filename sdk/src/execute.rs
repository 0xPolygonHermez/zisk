use std::time::Duration;

use anyhow::Result;

use super::client::ProverClient;
use crate::GuestProgram;
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
pub struct ExecuteRequest<'a> {
    client: &'a ProverClient,
    program: &'a GuestProgram,
    stdin: ZiskStdin,
    timeout: Option<Duration>,
    traces: Vec<Tracing>,
}

impl<'a> ExecuteRequest<'a> {
    pub(crate) fn new(
        client: &'a ProverClient,
        program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> Self {
        Self { client, program, stdin, timeout: None, traces: Vec::new() }
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
        todo!()
    }
}
