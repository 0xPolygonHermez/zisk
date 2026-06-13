use crate::upload::UploadResult;

use super::RemoteClient;
use zisk_prover_backend::GuestProgram;

use crate::{Result, SdkError};

impl RemoteClient {
    /// Register a program, blocking the calling thread. Requires a live tokio runtime.
    pub fn do_upload(&self, program: &GuestProgram) -> Result<UploadResult> {
        let expected_hash_id = &program.program_id.hash_id;

        let computed_hash_id =
            self.gw.register_program(program.elf().to_vec()).map_err(SdkError::backend)?;

        if computed_hash_id != *expected_hash_id {
            return Err(SdkError::UnexpectedResponse(format!(
                "Coordinator returned hash_id '{}', expected '{}'. \
                 Ensure the program was compiled for the correct target.",
                computed_hash_id, expected_hash_id
            )));
        }

        Ok(UploadResult::new(computed_hash_id.to_string()))
    }
}
