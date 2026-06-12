use super::EmbeddedClient;
use crate::embedded::{EmbeddedProver, ERR_ASSEMBLY_NOT_ENABLED};
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
        let output = match (prover.as_ref(), executor) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator) => {
                if hints.is_some() {
                    return Err(SdkError::UnsupportedExecutor(
                        "Hints require Assembly executor".to_string(),
                    ));
                }
                if matches!(stdin, InputSource::Stream(_)) {
                    return Err(SdkError::UnsupportedExecutor("Stream stdin (quic://, unix://) is not supported with the Emulator executor — use Assembly executor".to_string()));
                }
                let InputSource::Stdin(s) = stdin else { unreachable!() };
                p.execute(program, s.into_inner()).map_err(SdkError::backend)?
            }
            (EmbeddedProver::Emu(_), ExecutorKind::Assembly) => {
                return Err(SdkError::UnsupportedExecutor(ERR_ASSEMBLY_NOT_ENABLED.to_string()))
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Emulator) => {
                if hints.is_some() {
                    return Err(SdkError::UnsupportedExecutor(
                        "Hints require Assembly executor".to_string(),
                    ));
                }
                if matches!(stdin, InputSource::Stream(_)) {
                    return Err(SdkError::UnsupportedExecutor("Stream stdin (quic://, unix://) is not supported with the Emulator executor — use Assembly executor".to_string()));
                }
                let InputSource::Stdin(s) = stdin else { unreachable!() };
                p.execute_emulator(program, s.into_inner()).map_err(SdkError::backend)?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly) => {
                if let Some(hints) = hints {
                    if !p.was_setup_with_hints() {
                        return Err(SdkError::InvalidConfig(
                            "Program was set up without hints — call setup().with_hints() first"
                                .to_string(),
                        ));
                    }
                    match hints {
                        HintsSource::Hints(h) => {
                            p.register_hints_stream(h.into_inner()).map_err(SdkError::backend)?;
                        }
                        HintsSource::Stream(stream) => {
                            if stream.is_grpc() {
                                return Err(SdkError::UnsupportedExecutor("gRPC streams are not supported with the embedded executor — use a remote client".to_string()));
                            }
                            stream.start()?;
                            let uri = stream.uri().to_string();
                            let source = StreamSource::from_uri(&uri).map_err(SdkError::backend)?;
                            p.register_hints_stream(source).map_err(SdkError::backend)?;
                        }
                    }
                    p.execute(program, zisk_common::io::ZiskStdin::new())
                        .map_err(SdkError::backend)?
                } else {
                    if p.was_setup_with_hints() {
                        return Err(SdkError::InvalidConfig(
                            "Program was set up with hints — call .hints() on the request"
                                .to_string(),
                        ));
                    }
                    match stdin {
                        InputSource::Stream(stream) => {
                            if stream.is_grpc() {
                                return Err(SdkError::UnsupportedExecutor("gRPC streams are not supported with the embedded executor — use a remote client".to_string()));
                            }
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
        };
        Ok(ExecuteResult::from(output))
    }
}
