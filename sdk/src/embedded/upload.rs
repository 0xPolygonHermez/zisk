use super::EmbeddedClient;
use crate::upload::UploadResult;
use anyhow::Result;
use zisk_prover_backend::GuestProgram;

impl EmbeddedClient {
    pub(crate) fn do_upload(&self, _program: &GuestProgram) -> Result<UploadResult> {
        // No upload step needed for embedded client — it has direct access to ELF files.
        Ok(UploadResult)
    }
}
