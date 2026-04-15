use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use crate::Client;

pub struct UploadResult;

/// Builder for a program upload request.
///
/// Obtain via `client.upload(&program)`.
///
/// - Embedded client: no-op (program is available locally).
/// - Remote client: registers the ELF with the gateway and verifies the `hash_id` matches.
pub struct UploadRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
}

#[allow(private_bounds)]
impl<'a, C: Client> UploadRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram) -> Self {
        Self { client, program }
    }

    /// Run the upload synchronously.
    pub fn run(self) -> Result<UploadResult> {
        self.client.run_upload(self.program)
    }
}
