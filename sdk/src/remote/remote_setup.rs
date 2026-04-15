use super::RemoteClient;
use crate::{
    job_handle::{check_completed, JobHandle, JobHandleInner, SubscriberList},
    setup::SetupResult,
};
use std::time::Duration;
use zisk_gateway_grpc_api::proto::{
    job_kind::Kind as GatewayKind, JobKind, SetupRequest as GatewaySetupRequest,
};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_setup(
        &self,
        program: &GuestProgram,
        _with_hints: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let hash_id = program.program_id.hash_id.to_string();
        let job_kind = JobKind { kind: Some(GatewayKind::Setup(GatewaySetupRequest { hash_id })) };

        let job_id = self.submit_job(job_kind)?;
        let gateway = self.gw_client.clone();
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
}
