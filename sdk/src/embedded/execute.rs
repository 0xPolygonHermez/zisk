use super::EmbeddedClient;
use crate::embedded::{EmbeddedProver, ERR_ASSEMBLY_NOT_ENABLED};
use crate::input::ProgramInput;
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::{ExecutorKind, JobEvent, ZiskStdin};
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use zisk_prover_backend::ExecuteOutput;
use zisk_prover_backend::GuestProgram;

impl EmbeddedClient {
    pub(crate) fn do_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteOutput>> {
        let program = program.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = Self::do_execute_inner(input, executor, program, prover)?;

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    fn do_execute_inner(
        input: ProgramInput,
        executor: ExecutorKind,
        program: GuestProgram,
        prover: Arc<EmbeddedProver>,
    ) -> Result<std::result::Result<ExecuteOutput, anyhow::Error>, anyhow::Error> {
        let result = {
            let result = match (prover.as_ref(), executor, input) {
                (EmbeddedProver::Emu(p), ExecutorKind::Emulator, ProgramInput::Stdin(stdin)) => {
                    p.execute(&program, stdin.into_inner())?
                }
                (EmbeddedProver::Emu(_), ExecutorKind::Emulator, ProgramInput::Hints(_)) => {
                    anyhow::bail!("Hints require Assembly executor")
                }
                (EmbeddedProver::Emu(_), ExecutorKind::Assembly, _) => {
                    anyhow::bail!(ERR_ASSEMBLY_NOT_ENABLED)
                }
                (EmbeddedProver::Asm(_), ExecutorKind::Emulator, _) => {
                    unimplemented!("Assembly prover does not yet support emulation mode")
                }
                (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Stdin(stdin)) => {
                    if p.was_setup_with_hints()? {
                        anyhow::bail!(
                            "Program was set up with hints — pass ZiskHints, not ZiskStdin"
                        );
                    }
                    p.execute(&program, stdin.into_inner())?
                }
                (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                    if !p.was_setup_with_hints()? {
                        anyhow::bail!(
                            "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                        );
                    }
                    p.register_hints_stream(hints.into_inner())?;
                    p.execute(&program, ZiskStdin::new().into_inner())?
                }
            };
            Ok(result)
        };
        Ok(result)
    }
}
