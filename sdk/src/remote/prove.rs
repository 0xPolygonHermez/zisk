use super::{job_inputs, JobInputs, RemoteClient};
use crate::{
    hints::HintsSource,
    input_source::InputSource,
    job_handle::{InputPushes, JobHandle, SubscriberList},
    prove::ProveResult,
    ExecutorKind,
};
use std::collections::BTreeMap;
use std::time::Duration;
use zisk_common::{ProofKind, RandomSeed};
use zisk_coordinator_api::dto::{deadline_from_now, DomainJobKind, DomainProveRequest};
use zisk_prover_backend::GuestProgram;

use crate::{Result, SdkError};

impl RemoteClient {
    /// Submit a prove job over the standard coordinator API.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_prove(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        seed: Option<RandomSeed>,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        self.submit_prove(program, stdin, hints, seed, executor, proof_kind, timeout, subs, None)
    }

    /// Submit a prove job over the extended coordinator API, attaching
    /// job-level `metadata` key/value pairs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_prove_ext(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        seed: Option<RandomSeed>,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
        metadata: BTreeMap<String, String>,
    ) -> Result<JobHandle<ProveResult>> {
        self.submit_prove(
            program,
            stdin,
            hints,
            seed,
            executor,
            proof_kind,
            timeout,
            subs,
            Some(metadata),
        )
    }

    /// Shared prove submission. `metadata` selects the transport: `Some` routes
    /// through the extended API (`submit_job_ext`), `None` through the standard one.
    #[allow(clippy::too_many_arguments)]
    fn submit_prove(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        seed: Option<RandomSeed>,
        _executor: ExecutorKind, // remote: coordinator uses its configured executor; hint ignored
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
        metadata: Option<BTreeMap<String, String>>,
    ) -> Result<JobHandle<ProveResult>> {
        let metadata = metadata.filter(|m| !m.is_empty());
        let JobInputs { input, input_delivery, hints, hints_delivery } = job_inputs(stdin, hints)?;

        let hash_id = program.program_id.hash_id.to_string();
        let proof_timeout = timeout.map(deadline_from_now);
        let proof_dest = proof_kind.into();

        // Prepare transports BEFORE submit_job().
        input_delivery.start()?;
        hints_delivery.start()?;

        let job_kind = DomainJobKind::Prove(DomainProveRequest {
            hash_id,
            input,
            hints,
            random_seed: seed,
            proof_timeout,
            proof_dest,
        });

        let remote_job = match metadata {
            Some(metadata) => self.gw.submit_job_ext(job_kind, metadata),
            None => self.gw.submit_job(job_kind),
        }
        .map_err(SdkError::backend)?;

        // gRPC streams need an InputSender injected after job submission, and data over the
        // inline limit is pushed through one, in the background.
        let mut pushes = InputPushes::default();
        let stream = input_delivery.complete(&remote_job, false, &mut pushes);
        let hints_stream = hints_delivery.complete(&remote_job, true, &mut pushes);

        let mut handle = JobHandle::new_remote(remote_job, subs, timeout, stream, hints_stream);
        handle.set_input_pushes(pushes);
        Ok(handle)
    }
}
