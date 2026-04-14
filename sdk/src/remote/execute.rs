use std::time::Duration;

use anyhow::Result;
use zisk_gateway_grpc_api::proto::{
    job_kind::Kind as GatewayKind, ExecuteRequest as GatewayExecuteRequest, JobKind,
};
use zisk_prover_backend::GuestProgram;

use super::{duration_to_proto_timestamp, stdin_to_input_kind, RemoteClient};
use crate::execute::ExecuteResult;
use crate::input::ProgramInput;
use crate::job_handle::{extract_execute, JobHandle, JobHandleInner, SubscriberList};
use crate::ExecutorKind;

pub(crate) fn run(
    remote: &RemoteClient,
    program: &GuestProgram,
    input: ProgramInput,
    _executor: ExecutorKind,
    timeout: Option<Duration>,
    subs: SubscriberList,
) -> Result<JobHandle<ExecuteResult>> {
    let hash_id = program.program_id.hash_id.to_string();
    let input_kind = stdin_to_input_kind(input)?;
    let execute_timeout = timeout.map(duration_to_proto_timestamp);
    
    let job_kind = JobKind {
        kind: Some(GatewayKind::Execute(GatewayExecuteRequest {
            hash_id,
            input: Some(input_kind),
            execute_timeout,
        })),
    };
    let job_id = remote.submit_job_sync(job_kind)?;
    let gateway = remote.gateway_client();
    Ok(JobHandle {
        inner: JobHandleInner::Remote { gateway, job_id, extract: Box::new(extract_execute) },
        subscribers: subs,
        timeout,
    })
}
