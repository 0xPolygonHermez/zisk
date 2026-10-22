use super::{hints_to_input_kind, stdin_to_input_kind, RemoteClient};

use crate::execute::ExecuteResult;
use crate::hints::HintsSource;
use crate::input_source::InputSource;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::ExecutorKind;
use crate::SdkError;

use std::collections::BTreeMap;
use std::time::Duration;
use zisk_common::RandomSeed;
use zisk_coordinator_api::dto::{deadline_from_now, DomainExecuteRequest, DomainJobKind};
use zisk_prover_backend::GuestProgram;

use crate::Result;

impl RemoteClient {
    /// Submit an execute job over the standard coordinator API.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_execute(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        seed: Option<RandomSeed>,
        executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteResult>> {
        self.submit_execute(program, stdin, hints, seed, executor, timeout, subs, None)
    }

    /// Submit an execute job over the extended coordinator API, attaching
    /// job-level `metadata` key/value pairs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_execute_ext(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        seed: Option<RandomSeed>,
        executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
        metadata: BTreeMap<String, String>,
    ) -> Result<JobHandle<ExecuteResult>> {
        self.submit_execute(program, stdin, hints, seed, executor, timeout, subs, Some(metadata))
    }

    /// Shared execute submission. `metadata` selects the transport: `Some` routes
    /// through the extended API (`submit_job_ext`), `None` through the standard one.
    #[allow(clippy::too_many_arguments)]
    fn submit_execute(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        seed: Option<RandomSeed>,
        _executor: ExecutorKind, // remote: coordinator uses its configured executor; hint ignored
        timeout: Option<Duration>,
        subs: SubscriberList,
        metadata: Option<BTreeMap<String, String>>,
    ) -> Result<JobHandle<ExecuteResult>> {
        let metadata = metadata.filter(|m| !m.is_empty());
        let (hints, hints_delivery) = hints_to_input_kind(hints)?;

        let hash_id = program.program_id.hash_id.to_string();
        let (input, input_delivery) = stdin_to_input_kind(stdin)?;
        let execute_timeout = timeout.map(deadline_from_now);

        // Prepare transports BEFORE submit_job().
        input_delivery.start()?;
        hints_delivery.start()?;

        let job_kind = DomainJobKind::Execute(DomainExecuteRequest {
            hash_id,
            input,
            hints,
            random_seed: seed,
            execute_timeout,
        });

        let remote_job = match metadata {
            Some(metadata) => self.gw.submit_job_ext(job_kind, metadata),
            None => self.gw.submit_job(job_kind),
        }
        .map_err(SdkError::backend)?;

        // gRPC streams need an InputSender injected after job submission, and data over the
        // inline limit is pushed through one.
        let stream = input_delivery.complete(&remote_job, false)?;
        let hints_stream = hints_delivery.complete(&remote_job, true)?;

        Ok(JobHandle::new_remote(remote_job, subs, timeout, stream, hints_stream))
    }
}
