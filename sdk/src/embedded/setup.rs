use std::time::Duration;

use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use super::{spawn_embedded_job, EmbeddedClient};
use crate::{
    job_handle::{JobHandle, SubscriberList},
    setup::SetupResult,
};

pub(crate) fn run(
    client: EmbeddedClient,
    program: &GuestProgram,
    with_hints: bool,
    timeout: Option<Duration>,
    subs: SubscriberList,
) -> Result<JobHandle<SetupResult>> {
    let program = program.clone();
    spawn_embedded_job(move || client.run_setup(&program, with_hints), timeout, subs)
}
