use super::{duration_to_proto_timestamp, proof_with_publics_to_proto, RemoteClient};
use crate::job_handle::{JobHandle, SubscriberList};
use std::time::Duration;
use zisk_common::{ProofKind, ZiskProofWithPublicValues};
use zisk_gateway_grpc_api::proto::{
    job_kind::Kind as GatewayKind, JobKind, WrapRequest as GatewayWrapRequest,
};

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ZiskProofWithPublicValues>> {
        let proto_proof = proof_with_publics_to_proto(proof_with_publics, proof_kind)?;
        let wrap_timeout = timeout.map(duration_to_proto_timestamp);
        let job_kind = JobKind {
            kind: Some(GatewayKind::Wrap(GatewayWrapRequest {
                proof: Some(proto_proof),
                proof_dest: proof_kind as i32,
                wrap_timeout,
            })),
        };
        let job_id = self.submit_job(job_kind)?;
        let gateway = self.gw_client.clone();

        Ok(JobHandle::new_remote(gateway, job_id, subs, timeout))
    }
}
