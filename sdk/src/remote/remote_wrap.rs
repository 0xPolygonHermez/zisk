use super::{duration_to_proto_timestamp, proof_with_publics_to_proto, RemoteClient};
use crate::job_handle::{extract_wrap, JobHandle, JobHandleInner, SubscriberList};
use std::time::Duration;
use zisk_common::{ProofMode, ZiskProofWithPublicValues};
use zisk_gateway_grpc_api::proto::{
    job_kind::Kind as GatewayKind, JobKind, ProofKind as GatewayProofKind,
    WrapRequest as GatewayWrapRequest,
};

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        mode: ProofMode,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ZiskProofWithPublicValues>> {
        let proof_dest = match mode {
            ProofMode::Plonk => GatewayProofKind::Plonk,
            ProofMode::VadcopFinal => GatewayProofKind::Stark,
            ProofMode::VadcopFinalMinimal => GatewayProofKind::StarkMinimal,
        };
        let proto_proof = proof_with_publics_to_proto(proof_with_publics, proof_dest)?;
        let wrap_timeout = timeout.map(duration_to_proto_timestamp);
        let job_kind = JobKind {
            kind: Some(GatewayKind::Wrap(GatewayWrapRequest {
                proof: Some(proto_proof),
                proof_dest: proof_dest as i32,
                wrap_timeout,
            })),
        };
        let job_id = self.submit_job(job_kind)?;
        let gateway = self.gw_client.clone();
        Ok(JobHandle {
            inner: JobHandleInner::Remote { gateway, job_id, extract: Box::new(extract_wrap) },
            subscribers: subs,
            timeout,
        })
    }
}
