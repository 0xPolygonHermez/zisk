use std::sync::Arc;
use std::time::Duration;

use zisk_common::io::StreamSource;
use zisk_prover_backend::{GuestProgram, VerifyConstraintsOutput};

use super::{EmbeddedClient, EmbeddedProver, ERR_ASSEMBLY_NOT_ENABLED};
use crate::hints::HintsSource;
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::prove::JobEvent;
use crate::verify_constraints::{
    RunVerifyConstraints, VerifyConstraintsExtension, VerifyConstraintsResult,
};
use crate::{ExecutorKind, ZiskStdin};
use crate::{Result, SdkError};

impl RunVerifyConstraints for EmbeddedClient {
    fn run_verify_constraints(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        hints: Option<HintsSource>,
        debug_info: Option<Option<String>>,
        executor: Option<ExecutorKind>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<VerifyConstraintsResult>> {
        let program = program.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();
        // Default to the executor the client was built with.
        let executor = executor.unwrap_or(self.executor);

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = Self::do_verify_constraints_inner(
                program, stdin, hints, debug_info, executor, prover,
            )
            .map(VerifyConstraintsResult::from);

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }
}

impl EmbeddedClient {
    fn do_verify_constraints_inner(
        program: GuestProgram,
        stdin: ZiskStdin,
        hints: Option<HintsSource>,
        debug_info: Option<Option<String>>,
        executor: ExecutorKind,
        prover: Arc<EmbeddedProver>,
    ) -> Result<VerifyConstraintsOutput> {
        match prover.as_ref() {
            EmbeddedProver::Emu(p) => {
                // The Emu prover has no assembly backend to switch to.
                if executor == ExecutorKind::Assembly {
                    return Err(SdkError::UnsupportedExecutor(
                        ERR_ASSEMBLY_NOT_ENABLED.to_string(),
                    ));
                }
                if hints.is_some() {
                    return Err(SdkError::UnsupportedExecutor(
                        "Hints require Assembly executor".to_string(),
                    ));
                }
                p.verify_constraints(&program, stdin.into_inner(), debug_info)
                    .map_err(SdkError::backend)
            }
            EmbeddedProver::Asm(p) => {
                // The Rust-emulator verify path does not consume hints.
                if executor == ExecutorKind::Emulator {
                    if hints.is_some() {
                        return Err(SdkError::UnsupportedExecutor(
                            "Hints require the Assembly executor".to_string(),
                        ));
                    }
                    return p
                        .verify_constraints_emulator(&program, stdin.into_inner(), debug_info)
                        .map_err(SdkError::backend);
                }

                match hints {
                    Some(hints) => {
                        if !p.was_setup_with_hints() {
                            return Err(SdkError::InvalidConfig(
                                "Program was set up without hints — call setup().with_hints() first"
                                    .to_string(),
                            ));
                        }
                        match hints {
                            HintsSource::Hints(h) => {
                                p.register_hints_stream(h.into_inner())
                                    .map_err(SdkError::backend)?;
                            }
                            HintsSource::Stream(stream) => {
                                if stream.is_grpc() {
                                    return Err(SdkError::UnsupportedExecutor("gRPC streams are not supported with the embedded executor — use a remote client".to_string()));
                                }
                                stream.start()?;
                                let uri = stream.uri().to_string();
                                let source =
                                    StreamSource::from_uri(&uri).map_err(SdkError::backend)?;
                                p.register_hints_stream(source).map_err(SdkError::backend)?;
                            }
                        }
                    }
                    None => {
                        if p.was_setup_with_hints() {
                            return Err(SdkError::InvalidConfig(
                                "Program was set up with hints — call .hints() on the request"
                                    .to_string(),
                            ));
                        }
                    }
                }
                p.verify_constraints(&program, stdin.into_inner(), debug_info)
                    .map_err(SdkError::backend)
            }
        }
    }
}

impl VerifyConstraintsExtension for EmbeddedClient {}
