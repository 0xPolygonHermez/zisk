use std::time::Duration;

use anyhow::Result;
use zisk_common::ProofMode;
use zisk_gateway_grpc_api::proto::{
    job_kind::Kind as GatewayKind, JobKind, ProveRequest as GatewayProveRequest,
};
use zisk_prover_backend::GuestProgram;

use super::{duration_to_proto_timestamp, stdin_to_input_kind, RemoteClient};
use crate::input::ProgramInput;
use crate::job_handle::{extract_prove, JobHandle, JobHandleInner, SubscriberList};
use crate::proof::Proof;
use crate::ExecutorKind;

pub(crate) fn run(
    remote: &RemoteClient,
    program: &GuestProgram,
    input: ProgramInput,
    _executor: ExecutorKind,
    _mode: ProofMode,
    timeout: Option<Duration>,
    subs: SubscriberList,
) -> Result<JobHandle<Proof>> {
    let hash_id = program.program_id.hash_id.to_string();
    let input_kind = stdin_to_input_kind(input)?;
    let proof_timeout = timeout.map(duration_to_proto_timestamp);
    let job_kind = JobKind {
        kind: Some(GatewayKind::Prove(GatewayProveRequest {
            hash_id,
            input: Some(input_kind),
            proof_timeout,
        })),
    };
    let job_id = remote.submit_job_sync(job_kind)?;
    let gateway = remote.gateway_client();
    Ok(JobHandle {
        inner: JobHandleInner::Remote { gateway, job_id, extract: Box::new(extract_prove) },
        subscribers: subs,
        timeout,
    })
}
