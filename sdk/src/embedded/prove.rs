use std::time::Duration;

use anyhow::Result;
use zisk_common::ProofKind;
use zisk_prover_backend::GuestProgram;

use super::{spawn_embedded_job, EmbeddedClient};
use crate::input::ProgramInput;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::proof::Proof;
use crate::ExecutorKind;

pub(crate) fn run(
    client: EmbeddedClient,
    program: &GuestProgram,
    input: ProgramInput,
    executor: ExecutorKind,
    proof_kind: ProofKind,
    timeout: Option<Duration>,
    subs: SubscriberList,
) -> Result<JobHandle<Proof>> {
    let program = program.clone();
    spawn_embedded_job(
        move || client.run_prove(&program, input, executor, proof_kind),
        timeout,
        subs,
    )
}
