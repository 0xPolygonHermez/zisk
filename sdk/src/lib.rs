mod aggregate_proofs;
mod cancel;
mod client;
mod embedded;
mod execute;
mod hints;
mod input_source;
mod input_stream;
mod job_handle;
mod lifecycle;
mod opts;
mod prove;
mod recurser;
mod remote;
mod setup;
mod stdin;
mod upload;
mod verify;
mod verify_constraints;
mod wrap;
mod zisk_client;

pub use aggregate_proofs::{AggregateProofsRequest, AggregationInput, ProofExt};
pub use cancel::CancellationToken;
pub use client::ProverClient;
pub use embedded::{
    EmbeddedClient, EmbeddedClientBuilder, EmbeddedExecuteOnlyBuilder, EmbeddedExecuteOnlyClient,
    WitnessBuilderExt,
};
pub use zisk_client::ZiskClient;

pub use execute::{ExecuteRequest, ExecuteResult};
pub use hints::{HintsSource, ZiskHints};
pub use input_source::InputSource;
pub use input_stream::ZiskStream;
pub use job_handle::JobHandle;
pub use lifecycle::{SetupTarget, UploadTarget};
pub use prove::{JobEvent, ProveRequest, ProveResult};
pub use recurser::{AggregationProgram, AggregationProgramBuilder, Recurser};
pub use remote::setup::SetupByIdRequest;
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
    load_circuit, load_program, Asm, AsmOptions, CircomCircuit, Elf, EmuOptions, GuestProgram,
    HashMode, ProfilingMode, ProgramId,
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

    fn run_upload_aggregation_program(
        &self,
        agg: &crate::recurser::Recurser,
    ) -> Result<UploadResult>;

    fn run_setup_aggregation_program(
        &self,
        agg: &crate::recurser::Recurser,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<SetupResult>>;

    fn run_aggregate_proofs(
        &self,
        agg: &crate::recurser::Recurser,
        proof_a: &Proof,
        proof_b: &Proof,
        free_inputs_a: &[u64],
        free_inputs_b: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<crate::prove::ProveResult>>;
}

/// Synchronous counterpart to [`Client`], implemented only by backends whose
/// work is genuinely synchronous (currently the embedded client).
///
/// Each method mirrors a [`Client`] `run_*` method but runs the job on the
/// calling thread and returns the result directly — no async runtime required.
/// Remote backends deliberately do not implement this: a remote call is network
/// I/O and has no honest synchronous form, so `run_sync()` is unavailable there
/// at compile time.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not support synchronous execution",
    note = "`run_sync()` is only available on `EmbeddedClient` — a `RemoteClient` \
            performs network I/O and has no synchronous form; use `run()` and \
            `.await` the returned `JobHandle` instead",
    label = "this client cannot run synchronously"
)]
#[allow(clippy::too_many_arguments)]
pub(crate) trait ClientSync {
    fn run_setup_sync(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
        subs: job_handle::SubscriberList,
    ) -> Result<SetupResult>;

    fn run_setup_aggregation_program_sync(
        &self,
        agg: &crate::recurser::Recurser,
        subs: job_handle::SubscriberList,
    ) -> Result<SetupResult>;

    fn run_prove_sync(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        proof_kind: ProofKind,
        subs: job_handle::SubscriberList,
    ) -> Result<ProveResult>;

    fn run_execute_sync(
        &self,
        program: &GuestProgram,
        stdin: InputSource,
        hints: Option<HintsSource>,
        executor: ExecutorKind,
        subs: job_handle::SubscriberList,
    ) -> Result<ExecuteResult>;

    fn run_wrap_sync(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<PublicValues>,
        override_program_vk: Option<ProgramVK>,
        subs: job_handle::SubscriberList,
    ) -> Result<crate::prove::ProveResult>;
}
