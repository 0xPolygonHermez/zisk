use std::ops::Deref;
use std::time::Duration;

use crate::Result;
use zisk_prover_backend::{ExecuteOutput, GuestProgram};

use crate::hints::HintsSource;
use crate::input_source::InputSource;
use crate::job_handle::{new_subscriber_list, JobHandle, JobId};
use crate::{Client, ClientSync, ExecutorKind};

pub struct ExecuteResult {
    job_id: Option<JobId>,
    output: ExecuteOutput,
}

impl ExecuteResult {
    pub fn new(output: ExecuteOutput, job_id: Option<JobId>) -> Self {
        Self { output, job_id }
    }

    pub fn job_id(&self) -> Option<&JobId> {
        self.job_id.as_ref()
    }

    pub fn output(&self) -> &ExecuteOutput {
        &self.output
    }
}

impl Deref for ExecuteResult {
    type Target = ExecuteOutput;
    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl From<ExecuteOutput> for ExecuteResult {
    fn from(output: ExecuteOutput) -> Self {
        Self { output, job_id: None }
    }
}

/// Builder for a dry-run execution request (no proof).
///
/// Obtain via `client.execute(&program, stdin)`.
pub struct ExecuteRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
    stdin: InputSource,
    hints: Option<HintsSource>,
    executor: ExecutorKind,
    timeout: Option<Duration>,
}

#[allow(private_bounds)]
impl<'a, C: Client> ExecuteRequest<'a, C> {
    pub(crate) fn new(
        client: &'a C,
        program: &'a GuestProgram,
        stdin: impl Into<InputSource>,
        executor: ExecutorKind,
    ) -> Self {
        Self { client, program, stdin: stdin.into(), hints: None, executor, timeout: None }
    }

    /// Attach a hints stream to this execute request.
    ///
    /// Requires the program to have been set up with
    /// [`SetupRequest::with_hints`](crate::SetupRequest::with_hints) and
    /// the [`ExecutorKind::Assembly`] executor.
    #[must_use]
    pub fn hints(mut self, hints: impl Into<HintsSource>) -> Self {
        self.hints = Some(hints.into());
        self
    }

    /// Override the executor for this execute call.
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = executor;
        self
    }

    /// Set a timeout for the execution.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Submit the execution, returning a [`JobHandle<ExecuteOutput>`].
    pub fn run(self) -> Result<JobHandle<ExecuteResult>> {
        let subs = new_subscriber_list();
        self.client.run_execute(
            self.program,
            self.stdin,
            self.hints,
            self.executor,
            self.timeout,
            subs,
        )
    }
}

#[allow(private_bounds)]
impl<'a, C: ClientSync> ExecuteRequest<'a, C> {
    /// Run the execution synchronously, returning the result directly.
    ///
    /// Unlike [`run`](Self::run), this drives the work on the calling thread and
    /// requires no async runtime — use it when embedding the SDK in a
    /// synchronous program. Available only for clients that implement
    /// [`ClientSync`] (the embedded client).
    pub fn run_sync(self) -> Result<ExecuteResult> {
        let subs = new_subscriber_list();
        self.client.run_execute_sync(self.program, self.stdin, self.hints, self.executor, subs)
    }
}
