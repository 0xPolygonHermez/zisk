use super::EmbeddedClient;
use crate::{
    embedded::EmbeddedProver,
    job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList},
    setup::SetupResult,
    JobEvent,
};
use crate::{Result, SdkError};
use std::{sync::Arc, time::Duration};
use zisk_prover_backend::GuestProgram;

impl EmbeddedClient {
    pub(crate) fn do_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let program = program.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = Self::do_setup_inner(with_hints, emulator_only, &program, prover);

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    /// Run ROM setup synchronously on the calling thread.
    ///
    /// Unlike [`do_setup`](Self::do_setup), this performs no `spawn_blocking`
    /// and returns the result directly, so it requires no async runtime.
    pub(crate) fn do_setup_sync(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
        subs: SubscriberList,
    ) -> Result<SetupResult> {
        fire_event(&subs, JobEvent::Started);
        let result = Self::do_setup_inner(with_hints, emulator_only, program, self.prover.clone());
        fire_result_event(&subs, &result);
        result
    }

    fn do_setup_inner(
        with_hints: bool,
        emulator_only: bool,
        program: &GuestProgram,
        prover: Arc<EmbeddedProver>,
    ) -> Result<SetupResult> {
        match prover.as_ref() {
            EmbeddedProver::Emu(p) => {
                p.setup(program).run().map_err(SdkError::backend)?;
            }
            EmbeddedProver::Asm(p) => {
                let mut builder = p.setup(program);
                if with_hints {
                    builder = builder.with_hints();
                }
                if emulator_only {
                    builder = builder.emulator_only();
                }
                builder.run().map_err(SdkError::backend)?;
            }
        }
        Ok(SetupResult { job_id: None })
    }
}
