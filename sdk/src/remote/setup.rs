use std::time::Duration;

use anyhow::Result;
use zisk_gateway_grpc_api::proto::{
    job_kind::Kind as GatewayKind, JobKind, SetupRequest as GatewaySetupRequest,
};
use zisk_prover_backend::GuestProgram;

use super::RemoteClient;
use crate::job_handle::{check_completed, JobHandle, JobHandleInner, SubscriberList};

pub(crate) fn run(
    remote: &RemoteClient,
    program: &GuestProgram,
    _with_hints: bool,
    timeout: Option<Duration>,
    subs: SubscriberList,
) -> Result<JobHandle<()>> {
    let hash_id = program.program_id.hash_id.to_string();
    let job_kind = JobKind { kind: Some(GatewayKind::Setup(GatewaySetupRequest { hash_id })) };
    let job_id = remote.submit_job_sync(job_kind)?;
    let gateway = remote.gateway_client();
    Ok(JobHandle {
        inner: JobHandleInner::Remote {
            gateway,
            job_id,
            extract: Box::new(|resp| check_completed(&resp)),
        },
        subscribers: subs,
        timeout,
    })
}
