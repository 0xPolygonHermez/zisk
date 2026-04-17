use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use zisk_prover_backend::{ExecuteOutput, GuestProgram};

use crate::job_handle::JobHandle;
use crate::{Client, ExecutorKind};

/// Builder for a dry-run execution request (no proof).
///
/// Obtain via `client.execute(&program, stdin)`.
pub struct ExecuteRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
    input: crate::input::ProgramInput,
    executor: Option<ExecutorKind>,
    timeout: Option<Duration>,
}

#[allow(private_bounds)]
impl<'a, C: Client> ExecuteRequest<'a, C> {
    pub(crate) fn new(
        client: &'a C,
        program: &'a GuestProgram,
        input: impl Into<crate::input::ProgramInput>,
    ) -> Self {
        Self { client, program, input: input.into(), executor: None, timeout: None }
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

    /// Submit the execution, returning a [`JobHandle<ExecuteOutput>`].
    pub fn run(self) -> Result<JobHandle<ExecuteOutput>> {
        let executor = self.executor.unwrap_or(ExecutorKind::Emulator);
        let subs = Arc::new(Mutex::new(Vec::new()));
        self.client.run_execute(self.program, self.input, executor, self.timeout, subs)
    }
}
