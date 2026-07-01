use super::{hints_to_input_kind, stdin_to_input_kind, RemoteClient};
use crate::{
    hints::HintsSource,
    input_source::InputSource,
    job_handle::{JobHandle, SubscriberList},
    prove::ProveResult,
    ExecutorKind,
};
use std::time::Duration;
use zisk_common::ProofKind;
use zisk_coordinator_api::dto::{deadline_from_now, DomainJobKind, DomainProveRequest};
use zisk_prover_backend::GuestProgram;

use crate::{Result, SdkError};

impl RemoteClient {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_prove(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        _executor: ExecutorKind, // remote: coordinator uses its configured executor; hint ignored
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        let (hints, maybe_hints_stream) = hints_to_input_kind(hints)?;

        let hash_id = program.program_id.hash_id.to_string();
        let (input, maybe_stream) = stdin_to_input_kind(stdin)?;
        let proof_timeout = timeout.map(deadline_from_now);
        let proof_dest = proof_kind.into();

        // Prepare transports BEFORE submit_job().
        if let Some(ref stream) = maybe_stream {
            stream.start()?;
        }
        if let Some(ref stream) = maybe_hints_stream {
            stream.start()?;
        }

        let job_kind = DomainJobKind::Prove(DomainProveRequest {
            hash_id,
            input,
            hints,
            proof_timeout,
            proof_dest,
        });

        let remote_job = self.gw.submit_job(job_kind).map_err(SdkError::backend)?;

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
