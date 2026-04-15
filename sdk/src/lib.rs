mod cancel;
mod client;
mod embedded;
mod execute;
mod hints;
mod input;
mod job_handle;
mod opts;
mod proof;
mod prove;
mod remote;
mod setup;
mod stdin;
mod upload;
mod wrap;

pub use cancel::CancellationToken;
pub use client::ProverClient;
pub use embedded::{EmbeddedClient, EmbeddedClientBuilder};
pub use execute::{ExecuteRequest, ExecuteResult};
pub use hints::ZiskHints;
pub use input::ProgramInput;
pub use job_handle::JobHandle;
pub use proof::Proof;
pub use prove::{JobEvent, ProofKind, ProveRequest};
pub use remote::{RemoteClient, RemoteClientBuilder};
pub use setup::SetupRequest;
pub use stdin::ZiskStdin;
pub use upload::UploadRequest;
pub use wrap::WrapRequest;

// Re-export guest types from backend (public API for loading programs)
pub use zisk_prover_backend::{load_program, Elf, EmuOptions, GuestProgram, ProgramId};

pub use opts::ProverOpts;

// Re-export result and data types from backend (public outputs)
pub use zisk_prover_backend::{
    AsmOptions, BackendProverOpts, ProveBuilder, WrapBuilder, ZiskExecuteResult, ZiskProveResult,
    ZiskVerifyConstraintsResult,
};

// Re-export common types
pub use proofman_common::VerboseMode;

// Re-export types from zisk_common
pub use zisk_common::{
    PlonkVkey, ProofMode, ZiskProgramVK, ZiskProof, ZiskProofWithPublicValues, ZiskPublics, ZiskVK,
};

pub use zisk_build::*;

use anyhow::Result;

use crate::{setup::SetupResult, upload::UploadResult};

pub(crate) fn validate_stream_uri(uri: &str) -> Result<()> {
    let is_valid = uri.starts_with("quic://") || (cfg!(unix) && uri.starts_with("unix://"));
    if !is_valid {
        #[cfg(unix)]
        anyhow::bail!("stream() requires 'quic://' or 'unix://' scheme. Got: '{}'", uri);
        #[cfg(not(unix))]
        anyhow::bail!(
            "stream() requires 'quic://' scheme. Got: '{}' (unix:// not supported on this platform)",
            uri
        );
    }
    Ok(())
}

/// Executor backend for running programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutorKind {
    /// Emulator: always available.
    #[default]
    Emulator,
    /// Assembly: must be explicitly enabled on the client builder.
    Assembly,
}

pub(crate) trait Client: Clone + Send + Sync + 'static {
    fn run_upload(&self, program: &GuestProgram) -> Result<UploadResult>;

    fn run_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<SetupResult>>;

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<Proof>>;

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<execute::ExecuteResult>>;

    fn run_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        mode: ProofMode,
        override_publics: Option<ZiskPublics>,
        override_program_vk: Option<ZiskProgramVK>,
        timeout: Option<std::time::Duration>,
        subs: job_handle::SubscriberList,
    ) -> Result<job_handle::JobHandle<ZiskProofWithPublicValues>>;
}
