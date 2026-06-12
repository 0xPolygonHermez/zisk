use anyhow::Result;

use crate::lifecycle::UploadTarget;
use crate::Client;

pub struct UploadResult {
    hash_id: String,
}

impl UploadResult {
    pub fn new(hash_id: String) -> Self {
        Self { hash_id }
    }

    pub fn hash_id(&self) -> &str {
        &self.hash_id
    }
}

/// Builder for a program or recurser upload request.
///
/// Obtain via `client.upload(&program)` or `client.upload(&recurser)`.
/// - Embedded: no-op (artifacts are already local).
/// - Remote: registers the program ELF or recurser spec on the coordinator.
pub struct UploadRequest<'a, C> {
    client: &'a C,
    target: UploadTarget<'a>,
}

#[allow(private_bounds)]
impl<'a, C: Client> UploadRequest<'a, C> {
    pub(crate) fn new(client: &'a C, target: UploadTarget<'a>) -> Self {
        Self { client, target }
    }

    /// Run the upload synchronously.
    pub fn run(self) -> Result<UploadResult> {
        match self.target {
            UploadTarget::Program(p) => self.client.run_upload(p),
            UploadTarget::Recurser(a) => self.client.run_upload_aggregation_program(a),
        }
    }
}
