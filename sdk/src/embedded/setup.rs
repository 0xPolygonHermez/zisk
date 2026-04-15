use super::EmbeddedClient;
use crate::{
    embedded::EmbeddedProver,
    job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList},
    setup::SetupResult,
    JobEvent,
};
use anyhow::Result;
use std::{sync::Arc, time::Duration};
use zisk_prover_backend::GuestProgram;

impl EmbeddedClient {
    pub(crate) fn do_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let program = program.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = Self::do_setup_inner(with_hints, program, prover)?;

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    fn do_setup_inner(
        with_hints: bool,
        program: GuestProgram,
        prover: Arc<EmbeddedProver>,
    ) -> Result<std::result::Result<SetupResult, anyhow::Error>, anyhow::Error> {
        let result = match prover.as_ref() {
            EmbeddedProver::Emu(p) => {
                p.setup(&program).run()?;
                Ok(SetupResult)
            }
            EmbeddedProver::Asm(p) => {
                let builder = p.setup(&program);
                if with_hints {
                    builder.with_hints().run()?;
                } else {
                    builder.run()?;
                }
                Ok(SetupResult)
            }
        };
        Ok(result)
    }
}
