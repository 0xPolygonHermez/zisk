use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use super::RemoteClient;

pub(crate) fn run(remote: &RemoteClient, program: &GuestProgram) -> Result<()> {
    let expected = &program.program_id.hash_id;
    let actual = remote.register_program_sync(program.elf())?;
    if actual != *expected {
        anyhow::bail!(
            "Gateway returned hash_id '{}', expected '{}'. \
             Ensure the program was compiled for the correct target.",
            actual,
            expected
        );
    }
    Ok(())
}
