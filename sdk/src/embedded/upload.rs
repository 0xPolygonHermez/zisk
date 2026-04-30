use super::EmbeddedClient;
use crate::upload::UploadResult;
use anyhow::Result;
use zisk_prover_backend::GuestProgram;

impl EmbeddedClient {
    pub(crate) fn do_upload(&self, program: &GuestProgram) -> Result<UploadResult> {
        Ok(UploadResult::new(program.program_id.hash_id.to_string()))
    }
}
