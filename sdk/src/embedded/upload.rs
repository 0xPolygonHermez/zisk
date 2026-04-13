use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use super::EmbeddedClient;

pub(crate) fn run(_client: &EmbeddedClient, _program: &GuestProgram) -> Result<()> {
    // No upload step needed for embedded client — it has direct access to ELF files.
    Ok(())
}
