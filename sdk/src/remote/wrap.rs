use super::RemoteClient;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::prove::ProveResult;
use std::time::Duration;
use zisk_common::{Proof, ProofKind};
use zisk_coordinator_api::dto::{deadline_from_now, DomainJobKind, DomainProof, DomainWrapRequest};

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        let data = bincode::serde::encode_to_vec(proof, bincode::config::standard())
            .map_err(|e| anyhow::anyhow!("failed to serialize proof: {e}"))?;

        // Derive a deterministic UUID from the serialized proof bytes so that retrying
        // the same wrap request produces the same ID (idempotent on the coordinator side).
        let proof_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &data);

        let proof = DomainProof {
            proof_id,
            hash_id: String::new(),       // coordinator fills this on wrap
            verification_key: Vec::new(), // coordinator fills this on wrap
            proof_kind: proof_kind.into(),
            data,
            public_inputs: Vec::new(), // coordinator fills this on wrap
            started_at: None,
            completed_at: None,
        };
        let proof_dest = proof.proof_kind.clone();
        let wrap_timeout = timeout.map(deadline_from_now);

        let job_kind = DomainJobKind::Wrap(DomainWrapRequest { proof, proof_dest, wrap_timeout });

        let remote_job = self.gw.submit_job(job_kind)?;

        Ok(JobHandle::new_remote(remote_job, subs, timeout, None, None))
    }
}
