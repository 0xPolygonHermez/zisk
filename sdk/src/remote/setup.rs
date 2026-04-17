use super::RemoteClient;
use crate::{
    job_handle::{JobHandle, SubscriberList},
    setup::SetupResult,
};
use std::time::Duration;
use zisk_gateway::backend::{DomainJobKind, DomainSetupRequest};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_setup(
        &self,
        program: &GuestProgram,
        _with_hints: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let hash_id = program.program_id.hash_id.to_string();
        let job_kind = DomainJobKind::Setup(DomainSetupRequest { hash_id });

        let job_id = self.submit_job(job_kind)?;
        let gateway = self.gw_client.clone();

        Ok(JobHandle::new_remote(gateway, job_id, subs, timeout))
    }
}
