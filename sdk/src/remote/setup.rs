use super::RemoteClient;
use crate::{
    job_handle::{JobHandle, SubscriberList},
    setup::SetupResult,
};
use std::time::Duration;
use zisk_coordinator_api::dto::{DomainJobKind, DomainSetupRequest};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let hash_id = program.program_id.hash_id.to_string();
        let program_name = program.program_id.name.to_string();
        let job_kind =
            DomainJobKind::Setup(DomainSetupRequest { hash_id, program_name, with_hints });

        let remote_job = self.gw.submit_job(job_kind)?;

        Ok(JobHandle::new_remote(remote_job, subs, timeout, None, None))
    }
}
