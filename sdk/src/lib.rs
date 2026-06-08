mod cancel;
mod client;
mod embedded;
mod execute;
mod hints;
mod input_source;
mod input_stream;
mod job_handle;
mod opts;
mod prove;
mod remote;
mod setup;
mod stdin;
mod upload;
mod verify;
mod verify_constraints;
mod wrap;

pub use cancel::CancellationToken;
pub use client::ProverClient;
pub use embedded::{
    EmbeddedClient, EmbeddedClientBuilder, EmbeddedExecuteOnlyBuilder, EmbeddedExecuteOnlyClient,
    WitnessBuilderExt,
};
pub use execute::{ExecuteRequest, ExecuteResult};
pub use hints::{HintsSource, ZiskHints};
pub use input_source::InputSource;
pub use input_stream::ZiskStream;
pub use job_handle::JobHandle;
pub use prove::{JobEvent, ProveRequest, ProveResult};
pub use remote::{RemoteClient, RemoteClientBuilder};
pub use setup::SetupRequest;
pub use stdin::ZiskStdin;
pub use upload::UploadRequest;
pub use verify::VerifyBuilder;
pub use verify_constraints::{
    VerifyConstraintsExtension, VerifyConstraintsRequest, VerifyConstraintsResult,
};
pub use wrap::WrapRequest;

// Re-export guest types from backend (public API for loading programs)
pub use zisk_prover_backend::{
    load_program, Asm, AsmOptions, Elf, EmuOptions, GuestProgram, ProfilingMode, ProgramId,
};

pub use opts::EmbeddedOpts;

// Re-export result and data types from backend (public outputs)
pub use zisk_prover_backend::{setup_logger, ExecuteOutput, ProveOutput, VerifyConstraintsOutput};

// Re-export common types
pub use proofman_common::VerboseMode;

// Re-export types from zisk_common
pub use zisk_common::{
    PlonkVkBlob, PlonkVkey, ProgramVK, Proof, ProofBody, ProofKind, PublicValues,
};

pub use zisk_build::*;

use anyhow::Result;

/// Run the ZisK emulator with the given program and stdin.
pub fn run(
    program: &GuestProgram,
    stdin: ZiskStdin,
    profiling: Option<ProfilingMode>,
) -> Result<()> {
    program.run_emulation(stdin.into_inner(), profiling)
}

use crate::{setup::SetupResult, upload::UploadResult};

/// Executor backend for running programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutorKind {
    /// Emulator: always available.
    #[default]
    Emulator,
    /// Assembly: must be explicitly enabled on the client builder.
    Assembly,
}

#[allow(clippy::too_many_arguments)]
pub(crate) trait Client: Clone + Send + Sync + 'static {
    /// Default executor configured on the client builder. Used when a
    /// per-call request does not explicitly override it.
    fn default_executor(&self) -> ExecutorKind {
        ExecutorKind::default()
    }

    fn run_upload(&self, program: &GuestProgram) -> Result<UploadResult>;

    fn run_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<SetupResult>>;

    fn run_prove(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<ProveResult>>;

    fn run_execute(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<ExecuteResult>>;

    fn run_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<PublicValues>,
        override_program_vk: Option<ProgramVK>,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<crate::prove::ProveResult>>;
}
