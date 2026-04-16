use crate::upload::UploadResult;

use super::RemoteClient;
use zisk_gateway_api::proto::RegisterGuestProgramRequest;
use zisk_prover_backend::GuestProgram;

use anyhow::{Context, Result};

impl RemoteClient {
    /// Register a program, blocking the calling thread. Requires a live tokio runtime.
    pub fn do_upload(&self, program: &GuestProgram) -> Result<UploadResult> {
        let expected_hash_id = &program.program_id.hash_id;

        let computed_hash_id = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.register_program(program))
        })?;

        if computed_hash_id != *expected_hash_id {
            anyhow::bail!(
                "Gateway returned hash_id '{}', expected '{}'. \
             Ensure the program was compiled for the correct target.",
                computed_hash_id,
                expected_hash_id
            );
        }
        Ok(UploadResult)
    }

    pub(crate) async fn register_program(&self, program: &GuestProgram) -> Result<String> {
        let mut gw = self.gw_client.clone();
        let resp = gw
            .register_guest_program(RegisterGuestProgramRequest {
                zisk_elf: program.elf().to_vec(),
            })
            .await
            .context("RegisterGuestProgram RPC failed")?;
        Ok(resp.into_inner().hash_id)
    }
}
