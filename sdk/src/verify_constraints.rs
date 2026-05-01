use std::ops::Deref;
use std::time::Duration;

use anyhow::Result;
use zisk_prover_backend::{GuestProgram, VerifyConstraintsOutput};

use crate::job_handle::{new_subscriber_list, JobHandle, JobId, SubscriberList};
use crate::ZiskStdin;

pub struct VerifyConstraintsResult {
    job_id: Option<JobId>,
    output: VerifyConstraintsOutput,
}

impl VerifyConstraintsResult {
    pub fn new(output: VerifyConstraintsOutput, job_id: Option<JobId>) -> Self {
        Self { output, job_id }
    }

    pub fn job_id(&self) -> Option<&JobId> {
        self.job_id.as_ref()
    }
}

impl Deref for VerifyConstraintsResult {
    type Target = VerifyConstraintsOutput;
    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl From<VerifyConstraintsOutput> for VerifyConstraintsResult {
    fn from(output: VerifyConstraintsOutput) -> Self {
        Self { output, job_id: None }
    }
}

pub(crate) trait RunVerifyConstraints {
    fn run_verify_constraints(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<VerifyConstraintsResult>>;
}

/// Builder for a verify-constraints request.
///
/// Obtain via `client.verify_constraints(&program, stdin)` after importing
/// [`VerifyConstraintsExtension`].
pub struct VerifyConstraintsRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
    stdin: ZiskStdin,
    /// `None` = no debug info; `Some(None)` = enable with default path;
    /// `Some(Some(path))` = enable with explicit output path.
    debug_info: Option<Option<String>>,
    timeout: Option<Duration>,
}

impl<'a, C> VerifyConstraintsRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram, stdin: ZiskStdin) -> Self {
        Self { client, program, stdin, debug_info: None, timeout: None }
    }

    /// Enable debug info output.
    ///
    /// Pass `None` to use the default output path, or `Some(path)` to specify one.
    #[must_use]
    pub fn debug_info(mut self, path: impl Into<Option<String>>) -> Self {
        self.debug_info = Some(path.into());
        self
    }

    /// Set a timeout for the verification.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }
}

#[allow(private_bounds)]
impl<'a, C: RunVerifyConstraints> VerifyConstraintsRequest<'a, C> {
    /// Submit the request, returning a [`JobHandle<VerifyConstraintsResult>`].
    pub fn run(self) -> Result<JobHandle<VerifyConstraintsResult>> {
        let subs = new_subscriber_list();
        self.client.run_verify_constraints(
            self.program,
            self.stdin,
            self.debug_info,
            self.timeout,
            subs,
        )
    }
}

/// Extension trait enabling verify-constraints calls on a client.
///
/// Import this trait to call `.verify_constraints()` on [`EmbeddedClient`](crate::EmbeddedClient).
///
/// **Not implemented for `RemoteClient`** — the coordinator does not yet support a
/// VerifyConstraints job kind. Use [`EmbeddedClient`](crate::EmbeddedClient) instead.
///
/// # Example
///
/// ```rust,ignore
/// use zisk_sdk::{EmbeddedClientBuilder, load_program, ZiskStdin, VerifyConstraintsExtension};
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = EmbeddedClientBuilder::default().build()?;
/// let program = load_program!("program.elf");
/// let stdin = ZiskStdin::new();
/// let result = client.verify_constraints(&program, stdin).run()?.await?;
/// println!("steps: {}", result.get_execution_steps());
/// # Ok(())
/// # }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not support verify_constraints",
    note = "verify_constraints is only available on `EmbeddedClient` — \
            `RemoteClient` cannot run this operation because the coordinator \
            does not yet have a VerifyConstraints job kind",
    label = "consider using `EmbeddedClientBuilder::new().build()` instead"
)]
#[allow(private_bounds)]
pub trait VerifyConstraintsExtension: RunVerifyConstraints + Sized {
    fn verify_constraints<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> VerifyConstraintsRequest<'a, Self> {
        VerifyConstraintsRequest::new(self, program, stdin)
    }
}
