use super::{deadline_from_now, stdin_to_input_kind, RemoteClient};
use crate::{
    input::ProgramInput,
    job_handle::{JobHandle, SubscriberList},
    ExecutorKind,
};
use std::time::Duration;
use zisk_common::ProofKind;
use zisk_gateway::backend::{DomainJobKind, DomainProveRequest};
use zisk_prover_backend::{GuestProgram, ProveOutput};

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        _executor: ExecutorKind,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveOutput>> {
        let hash_id = program.program_id.hash_id.to_string();
        let input = stdin_to_input_kind(input)?;
        let proof_timeout = timeout.map(deadline_from_now);
        let proof_dest = proof_kind.into();

        let job_kind =
            DomainJobKind::Prove(DomainProveRequest { hash_id, input, proof_timeout, proof_dest });

        let job_id = self.submit_job(job_kind)?;
        let gateway = self.gw_client.clone();

        Ok(JobHandle::new_remote(gateway, job_id, subs, timeout))
    }
}
