use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use super::RemoteClient;

pub(crate) fn run(remote: &RemoteClient, program: &GuestProgram) -> Result<()> {
    let expected_hash_id = &program.program_id.hash_id;
    let computed_hash_id = remote.register_program_sync(&program)?;
    if computed_hash_id != *expected_hash_id {
        anyhow::bail!(
            "Gateway returned hash_id '{}', expected '{}'. \
             Ensure the program was compiled for the correct target.",
            computed_hash_id,
            expected_hash_id
        );
    }
    Ok(())
}
