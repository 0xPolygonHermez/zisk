use super::{requested_snark, RemoteClient};
use crate::job_handle::{JobHandle, SubscriberList};
use crate::prove::ProveResult;
use std::collections::BTreeMap;
use std::time::Duration;
use zisk_common::{Proof, ProofKind};
use zisk_coordinator_api::dto::{deadline_from_now, DomainJobKind, DomainProof, DomainWrapRequest};

use crate::{Result, SdkError};

impl RemoteClient {
    pub(crate) fn do_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
        metadata: BTreeMap<String, String>,
    ) -> Result<JobHandle<ProveResult>> {
        let data = proof
            .to_bincode()
            .map_err(|e| SdkError::Serialization(format!("failed to serialize proof: {e}")))?;

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
            random_seed: None, // not used in requests; the wrap keeps the seed inside `data`
        };
        let proof_dest = proof.proof_kind.clone();
        let wrap_timeout = timeout.map(deadline_from_now);

        let job_kind = DomainJobKind::Wrap(DomainWrapRequest {
            proof,
            proof_dest,
            snark_protocol: requested_snark(self.snark, proof_kind),
            wrap_timeout,
        });

        let remote_job =
            self.gw.submit_job_with_metadata(job_kind, metadata).map_err(SdkError::backend)?;

        let mut handle = JobHandle::new_remote(remote_job, subs, timeout, None, None);
        self.check_snark(&mut handle, proof_kind);
        Ok(handle)
    }
}
