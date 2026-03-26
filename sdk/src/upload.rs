use anyhow::Result;

use crate::GuestProgram;

/// Builder for a program upload request.
///
/// Obtain via `client.upload(&program)`.
///
/// - Embedded client: no-op (program is already available locally).
/// - Remote client: uploads the ELF and registers the program on the coordinator.
#[allow(dead_code)]
pub struct UploadRequest<'a, C> {
    _client: &'a C,
    _program: &'a GuestProgram,
}

impl<'a, C> UploadRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram) -> Self {
        Self { _client: client, _program: program }
    }

    /// Run the upload.
    pub fn run(self) -> Result<()> {
        todo!()
    }
}
