use super::EmbeddedClient;
use crate::embedded::{EmbeddedProver, ERR_ASSEMBLY_NOT_ENABLED};
use crate::execute::ExecuteResult;
use crate::hints::HintsSource;
use crate::input_source::InputSource;
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::{ExecutorKind, JobEvent};
use anyhow::Result;
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

            let result = Self::do_execute_inner(stdin, hints, executor, program, prover);

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    fn do_execute_inner(
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        program: GuestProgram,
        prover: Arc<EmbeddedProver>,
    ) -> Result<ExecuteResult> {
        let output = match (prover.as_ref(), executor) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator) => {
                if hints.is_some() {
                    anyhow::bail!("Hints require Assembly executor");
                }
                if matches!(stdin, InputSource::Stream(_)) {
                    anyhow::bail!("Stream stdin (quic://, unix://) is not supported with the Emulator executor — use Assembly executor");
                }
                let InputSource::Stdin(s) = stdin else { unreachable!() };
                p.execute(&program, s.into_inner())?
            }
            (EmbeddedProver::Emu(_), ExecutorKind::Assembly) => {
                anyhow::bail!(ERR_ASSEMBLY_NOT_ENABLED)
            }
            (EmbeddedProver::Asm(_), ExecutorKind::Emulator) => {
                unimplemented!("Assembly prover does not yet support emulation mode")
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly) => {
                if let Some(hints) = hints {
                    if !p.was_setup_with_hints()? {
                        anyhow::bail!(
                            "Program was set up without hints — call setup().with_hints() first"
                        );
                    }
                    match hints {
                        HintsSource::Hints(h) => {
                            p.register_hints_stream(h.into_inner())?;
                        }
                        HintsSource::Stream(stream) => {
                            if stream.is_grpc() {
                                anyhow::bail!("gRPC streams are not supported with the embedded executor — use a remote client");
                            }
                            stream.start()?;
                            let uri = stream.uri().to_string();
                            let source = StreamSource::from_uri(&uri)?;
                            p.register_hints_stream(source)?;
                        }
                    }
                    p.execute(&program, zisk_common::io::ZiskStdin::new())?
                } else {
                    if p.was_setup_with_hints()? {
                        anyhow::bail!(
                            "Program was set up with hints — call .hints() on the request"
                        );
                    }
                    match stdin {
                        InputSource::Stream(stream) => {
                            if stream.is_grpc() {
                                anyhow::bail!("gRPC streams are not supported with the embedded executor — use a remote client");
                            }
                            stream.start()?;
                            let uri = stream.uri().to_string();
                            let source = StreamSource::from_uri(&uri)?;
                            p.register_inputs_stream(source)?;
                            p.execute(&program, zisk_common::io::ZiskStdin::new())?
                        }
                        InputSource::Stdin(s) => p.execute(&program, s.into_inner())?,
                    }
                }
            }
        };
        Ok(ExecuteResult::from(output))
    }
}
