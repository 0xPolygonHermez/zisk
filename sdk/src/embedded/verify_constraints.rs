use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use zisk_prover_backend::GuestProgram;

use super::{EmbeddedClient, EmbeddedProver};
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::prove::JobEvent;
use crate::verify_constraints::{
    RunVerifyConstraints, VerifyConstraintsExtension, VerifyConstraintsResult,
};
use crate::ZiskStdin;

impl RunVerifyConstraints for EmbeddedClient {
    fn run_verify_constraints(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<VerifyConstraintsResult>> {
        let program = program.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = match prover.as_ref() {
                EmbeddedProver::Emu(p) => {
                    p.verify_constraints(&program, stdin.into_inner(), debug_info)
                }
                EmbeddedProver::Asm(p) => {
                    p.verify_constraints(&program, stdin.into_inner(), debug_info)
                }
            }
            .map(VerifyConstraintsResult::from);

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }
}

impl VerifyConstraintsExtension for EmbeddedClient {}
