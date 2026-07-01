use super::EmbeddedClient;
use crate::embedded::{validate_embedded_request, EmbeddedProver, HintsKind, StdinKind};
use crate::execute::ExecuteResult;
use crate::hints::HintsSource;
use crate::input_source::InputSource;
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::{ExecutorKind, JobEvent, Result, SdkError};
use std::sync::Arc;
use std::time::Duration;
use zisk_common::io::StreamSource;
use zisk_prover_backend::GuestProgram;

impl EmbeddedClient {
    pub(crate) fn do_execute(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteResult>> {
        let program = program.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = Self::do_execute_inner(stdin, hints, executor, &program, prover);

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    /// Run the execution synchronously on the calling thread.
    ///
    /// Unlike [`do_execute`](Self::do_execute), this performs no `spawn_blocking`
    /// and returns the result directly, so it requires no async runtime.
    pub(crate) fn do_execute_sync(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        subs: SubscriberList,
    ) -> Result<ExecuteResult> {
        fire_event(&subs, JobEvent::Started);
        let result = Self::do_execute_inner(stdin, hints, executor, program, self.prover.clone());
        fire_result_event(&subs, &result);
        result
    }

    fn do_execute_inner(
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        program: &GuestProgram,
        prover: Arc<EmbeddedProver>,
    ) -> Result<ExecuteResult> {
        let (prover_is_asm, was_setup_with_hints) = match prover.as_ref() {
            EmbeddedProver::Asm(p) => (true, p.was_setup_with_hints()),
            EmbeddedProver::Emu(_) => (false, false),
        };
        validate_embedded_request(
            prover_is_asm,
            executor,
            hints.as_ref().map(HintsKind::of),
            StdinKind::of(&stdin),
            was_setup_with_hints,
        )?;

        // Inputs are validated above; the dispatch only routes valid requests.
        let output = match (prover.as_ref(), executor) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator) => {
                let InputSource::Stdin(s) = stdin else { unreachable!() };
                p.execute(program, s.into_inner()).map_err(SdkError::backend)?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Emulator) => {
                let InputSource::Stdin(s) = stdin else { unreachable!() };
                p.execute_emulator(program, s.into_inner()).map_err(SdkError::backend)?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly) => {
                if let Some(hints) = hints {
                    match hints {
                        HintsSource::Hints(h) => {
                            p.register_hints_stream(h.into_inner()).map_err(SdkError::backend)?;
                        }
                        HintsSource::Stream(stream) => {
                            stream.start()?;
                            let uri = stream.uri().to_string();
                            let source = StreamSource::from_uri(&uri).map_err(SdkError::backend)?;
                            p.register_hints_stream(source).map_err(SdkError::backend)?;
                        }
                    }
                    p.execute(program, zisk_common::io::ZiskStdin::new())
                        .map_err(SdkError::backend)?
                } else {
                    match stdin {
                        InputSource::Stream(stream) => {
                            stream.start()?;
                            let uri = stream.uri().to_string();
                            let source = StreamSource::from_uri(&uri).map_err(SdkError::backend)?;
                            p.register_inputs_stream(source).map_err(SdkError::backend)?;
                            p.execute(program, zisk_common::io::ZiskStdin::new())
                                .map_err(SdkError::backend)?
                        }
                        InputSource::Stdin(s) => {
                            p.execute(program, s.into_inner()).map_err(SdkError::backend)?
                        }
                    }
                }
            }
            (EmbeddedProver::Emu(_), ExecutorKind::Assembly) => {
                unreachable!("rejected by validate_embedded_request")
            }
        };
        Ok(ExecuteResult::from(output))
    }
}
