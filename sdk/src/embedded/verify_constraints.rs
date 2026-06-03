use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use zisk_common::io::StreamSource;
use zisk_prover_backend::{GuestProgram, VerifyConstraintsOutput};

use super::{EmbeddedClient, EmbeddedProver};
use crate::hints::HintsSource;
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
        hints: Option<HintsSource>,
        debug_info: Option<Option<String>>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<VerifyConstraintsResult>> {
        let program = program.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result =
                Self::do_verify_constraints_inner(program, stdin, hints, debug_info, prover)
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
        prover: Arc<EmbeddedProver>,
    ) -> Result<VerifyConstraintsOutput> {
        match prover.as_ref() {
            EmbeddedProver::Emu(p) => {
                if hints.is_some() {
                    anyhow::bail!("Hints require Assembly executor");
                }
                p.verify_constraints(&program, stdin.into_inner(), debug_info)
            }
            EmbeddedProver::Asm(p) => {
                match hints {
                    Some(hints) => {
                        if !p.was_setup_with_hints() {
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
                    }
                    None => {
                        if p.was_setup_with_hints() {
                            anyhow::bail!(
                                "Program was set up with hints — call .hints() on the request"
                            );
                        }
                    }
                }
                p.verify_constraints(&program, stdin.into_inner(), debug_info)
            }
        }
    }
}

impl VerifyConstraintsExtension for EmbeddedClient {}
