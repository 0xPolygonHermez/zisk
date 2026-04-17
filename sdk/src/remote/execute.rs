use super::{deadline_from_now, stdin_to_input_kind, RemoteClient};

use crate::input::ProgramInput;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::ExecutorKind;
use zisk_prover_backend::ExecuteOutput;

use std::time::Duration;
use zisk_gateway::backend::{DomainExecuteRequest, DomainJobKind};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        _executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteOutput>> {
        let hash_id = program.program_id.hash_id.to_string();
        let input = stdin_to_input_kind(input)?;
        let execute_timeout = timeout.map(deadline_from_now);

        let job_kind =
            DomainJobKind::Execute(DomainExecuteRequest { hash_id, input, execute_timeout });

        let job_id = self.submit_job(job_kind)?;
        let gateway = self.gw_client.clone();

        Ok(JobHandle::new_remote(gateway, job_id, subs, timeout))
    }
}
