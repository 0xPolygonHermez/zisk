use super::EmbeddedClient;
use crate::embedded::{EmbeddedProver, ERR_ASSEMBLY_NOT_ENABLED};
use crate::input::ProgramInput;
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::prove::ProveResult;
use crate::{ExecutorKind, JobEvent, ZiskStdin};
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use zisk_common::ProofKind;
use zisk_prover_backend::GuestProgram;

impl EmbeddedClient {
    pub(crate) fn do_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        let program = program.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = Self::do_prove_inner(prover, &program, input, executor, proof_kind);

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    fn do_prove_inner(
        prover: Arc<EmbeddedProver>,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        proof_kind: ProofKind,
    ) -> Result<ProveResult> {
        macro_rules! apply_mode {
            ($builder:expr) => {
                match proof_kind {
                    ProofKind::VadcopFinal => $builder,
                    ProofKind::VadcopFinalMinimal => {
                        $builder.wrap_proof(ProofKind::VadcopFinalMinimal)
                    }
                    ProofKind::Plonk => $builder.wrap_proof(ProofKind::Plonk),
                }
            };
        }
        let result = match (prover.as_ref(), executor, input) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator, ProgramInput::Stdin(stdin)) => {
                apply_mode!(p.prove(program, stdin.into_inner())).run()?
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
                    anyhow::bail!("Program was set up with hints — pass ZiskHints, not ZiskStdin");
                }
                apply_mode!(p.prove(program, stdin.into_inner())).run()?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                if !p.was_setup_with_hints()? {
                    anyhow::bail!(
                        "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                    );
                }
                p.register_hints_stream(hints.into_inner())?;
                apply_mode!(p.prove(program, ZiskStdin::new().into_inner())).run()?
            }
        };
        Ok(ProveResult::from(result))
    }
}
