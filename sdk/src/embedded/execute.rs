use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use super::{spawn_embedded_job, EmbeddedClient};
use crate::execute::ExecuteResult;
use crate::input::ProgramInput;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::ExecutorKind;

pub(crate) fn run(
    client: Arc<EmbeddedClient>,
    program: &GuestProgram,
    input: ProgramInput,
    executor: ExecutorKind,
    timeout: Option<Duration>,
    subs: SubscriberList,
) -> Result<JobHandle<ExecuteResult>> {
    let program = program.clone();
    spawn_embedded_job(move || client.run_execute(&program, input, executor), timeout, subs)
}
