use super::{stdin_to_input_kind, RemoteClient};

use crate::execute::ExecuteResult;
use crate::input::ProgramInput;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::ExecutorKind;

use std::time::Duration;
use zisk_gateway_api::dto::{deadline_from_now, DomainExecuteRequest, DomainJobKind};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        _executor: ExecutorKind, // remote: gateway uses its configured executor; hint ignored
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteResult>> {
        let hash_id = program.program_id.hash_id.to_string();
        let input = stdin_to_input_kind(input)?;
        let execute_timeout = timeout.map(deadline_from_now);

        let job_kind =
            DomainJobKind::Execute(DomainExecuteRequest { hash_id, input, execute_timeout });

        let remote_job = self.gw.submit_job(job_kind)?;

        Ok(JobHandle::new_remote(remote_job, subs, timeout))
    }
}
