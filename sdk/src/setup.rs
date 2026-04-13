use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use crate::job_handle::JobHandle;
use crate::Client;

/// Builder for a program ROM setup request.
///
/// Obtain via `client.setup(&program)`.
///
/// - Embedded client: executes ROM setup locally if not already done.
/// - Remote client: registers the program on the gateway for proving.
pub struct SetupRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
    with_hints: bool,
    timeout: Option<Duration>,
}

#[allow(private_bounds)]
impl<'a, C: Client> SetupRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram) -> Self {
        Self { client, program, with_hints: false, timeout: None }
    }

    /// Enable hints during ROM setup. Requires Assembly executor on the client builder.
    #[must_use]
    pub fn with_hints(mut self) -> Self {
        self.with_hints = true;
        self
    }

    /// Set a timeout for the setup job.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Submit the setup, returning a [`JobHandle<()>`].
    pub fn run(self) -> Result<JobHandle<()>> {
        let subs = Arc::new(Mutex::new(Vec::new()));
        self.client.run_setup(self.program, self.with_hints, self.timeout, subs)
    }
}
