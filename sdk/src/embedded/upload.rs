use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use crate::upload::UploadResult;

use super::EmbeddedClient;

pub(crate) fn run(_client: &EmbeddedClient, _program: &GuestProgram) -> Result<UploadResult> {
    // No upload step needed for embedded client — it has direct access to ELF files.
    Ok(UploadResult)
}
