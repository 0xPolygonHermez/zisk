use anyhow::Result;

use crate::{Client, GuestProgram};

/// Builder for a program upload request.
///
/// Obtain via `client.upload(&program)`.
///
/// - Embedded client: no-op (program is already available locally).
/// - Remote client: uploads the ELF and registers the program on the coordinator.
#[allow(dead_code)]
pub struct UploadRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
}

#[allow(private_bounds)]
impl<'a, C: Client> UploadRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram) -> Self {
        Self { client, program }
    }

    /// Run the upload.
    pub fn run(self) -> Result<()> {
        self.client.run_upload(self.program)
    }
}
