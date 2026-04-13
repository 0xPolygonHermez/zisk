use std::time::Duration;

use anyhow::Result;
use zisk_common::{ProofMode, ZiskProofWithPublicValues};
use zisk_gateway_grpc_api::proto::{
    job_kind::Kind as GatewayKind, JobKind, ProofKind as GatewayProofKind,
    WrapRequest as GatewayWrapRequest,
};

use super::{duration_to_proto_timestamp, proof_with_publics_to_proto, RemoteClient};
use crate::job_handle::{extract_wrap, JobHandle, JobHandleInner, SubscriberList};

pub(crate) fn run(
    remote: &RemoteClient,
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
    let job_id = remote.submit_job_sync(job_kind)?;
    let gateway = remote.gateway_client();
    Ok(JobHandle {
        inner: JobHandleInner::Remote { gateway, job_id, extract: Box::new(extract_wrap) },
        subscribers: subs,
        timeout,
    })
}
