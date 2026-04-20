use super::{stdin_to_input_kind, RemoteClient};
use crate::{
    input::ProgramInput,
    job_handle::{JobHandle, SubscriberList},
    prove::ProveResult,
    ExecutorKind,
};
use std::time::Duration;
use zisk_common::ProofKind;
use zisk_gateway_api::dto::{deadline_from_now, DomainJobKind, DomainProveRequest};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        _executor: ExecutorKind, // remote: gateway uses its configured executor; hint ignored
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        let hash_id = program.program_id.hash_id.to_string();
        let input = stdin_to_input_kind(input)?;
        let proof_timeout = timeout.map(deadline_from_now);
        let proof_dest = proof_kind.into();

        let job_kind =
            DomainJobKind::Prove(DomainProveRequest { hash_id, input, proof_timeout, proof_dest });

        let remote_job = self.gw.submit_job(job_kind)?;

        Ok(JobHandle::new_remote(remote_job, subs, timeout))
    }
}
