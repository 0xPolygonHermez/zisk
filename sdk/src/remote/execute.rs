use super::{hints_to_input_kind, stdin_to_input_kind, RemoteClient};

use crate::execute::ExecuteResult;
use crate::hints::HintsSource;
use crate::input_source::InputSource;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::ExecutorKind;

use std::time::Duration;
use zisk_coordinator_api::dto::{deadline_from_now, DomainExecuteRequest, DomainJobKind};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;

impl RemoteClient {
    pub(crate) fn do_execute(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        _executor: ExecutorKind, // remote: coordinator uses its configured executor; hint ignored
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteResult>> {
        let (hints, maybe_hints_stream) = hints_to_input_kind(hints)?;

        let hash_id = program.program_id.hash_id.to_string();
        let (input, maybe_stream) = stdin_to_input_kind(stdin)?;
        let execute_timeout = timeout.map(deadline_from_now);

        // Prepare transports BEFORE submit_job().
        if let Some(ref stream) = maybe_stream {
            stream.start()?;
        }
        if let Some(ref stream) = maybe_hints_stream {
            stream.start()?;
        }

        let job_kind =
            DomainJobKind::Execute(DomainExecuteRequest { hash_id, input, hints, execute_timeout });

        let remote_job = self.gw.submit_job(job_kind)?;

        // gRPC streams need an InputSender injected after job submission.
        if let Some(ref stream) = maybe_stream {
            if stream.is_grpc() {
                stream.set_input_sender(remote_job.open_input_stream());
            }
        }
        if let Some(ref stream) = maybe_hints_stream {
            if stream.is_grpc() {
                stream.set_input_sender(remote_job.open_hints_stream());
            }
        }

        Ok(JobHandle::new_remote(remote_job, subs, timeout, maybe_stream, maybe_hints_stream))
    }
}
