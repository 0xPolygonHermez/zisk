use super::{deadline_from_now, proof_to_proto, RemoteClient};
use crate::job_handle::{JobHandle, SubscriberList};
use std::time::Duration;
use zisk_common::{Proof, ProofKind};
use zisk_gateway::backend::{DomainJobKind, DomainWrapRequest};
use zisk_prover_backend::ProveOutput;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveOutput>> {
        let proof = proof_to_proto(proof, proof_kind)?;
        let proof_dest = proof_kind.into();
        let wrap_timeout = timeout.map(deadline_from_now);

        let job_kind = DomainJobKind::Wrap(DomainWrapRequest { proof, proof_dest, wrap_timeout });

        let job_id = self.submit_job(job_kind)?;
        let gateway = self.gw_client.clone();

        Ok(JobHandle::new_remote(gateway, job_id, subs, timeout))
    }
}
