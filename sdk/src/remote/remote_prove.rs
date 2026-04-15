use super::{duration_to_proto_timestamp, stdin_to_input_kind, RemoteClient};
use crate::{
    input::ProgramInput,
    job_handle::{extract_prove, JobHandle, JobHandleInner, SubscriberList},
    proof::Proof,
    ExecutorKind,
};
use std::time::Duration;
use zisk_common::ProofMode;
use zisk_gateway_grpc_api::proto::{
    job_kind::Kind as GatewayKind, JobKind, ProveRequest as GatewayProveRequest,
};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_prove(
        &self,
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
        let job_id = self.submit_job(job_kind)?;
        let gateway = self.gw_client.clone();
        Ok(JobHandle {
            inner: JobHandleInner::Remote { gateway, job_id, extract: Box::new(extract_prove) },
            subscribers: subs,
            timeout,
        })
    }
}
