use anyhow::Result;

use super::types::ClientConfig;
use crate::GuestProgram;

/// Builder for a program ROM setup request.
///
/// Obtain via `client.setup(&program)`.
///
/// - Embedded client: executes ROM setup locally if not already done.
/// - Remote client: enables the program for proving on the coordinator.
#[allow(dead_code)]
pub struct SetupRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
    with_hints: bool,
}

impl<'a, C: ClientConfig> SetupRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram) -> Self {
        Self { client, program, with_hints: false }
    }

    /// Enable hints during ROM setup. Requires Assembly executor on the client builder.
    #[must_use]
    pub fn with_hints(mut self) -> Self {
        self.with_hints = true;
        self
    }

    /// Run the setup.
    pub fn run(self) -> Result<()> {
        if self.with_hints && !self.client.assembly_enabled() {
            anyhow::bail!(
                "Hints require Assembly executor — call .executor(Executor::Assembly) on the builder"
            );
        }
        todo!()
    }
}
