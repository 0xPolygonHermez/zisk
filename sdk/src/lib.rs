mod cancel;
mod client;
mod embedded;
mod execute;
mod hints;
mod input;
mod opts;
mod proof;
mod prove;
mod remote;
mod setup;
mod stdin;
mod upload;
mod wrap;

pub use cancel::CancellationToken;
pub use client::{ProverClient, ProverClientBuilder};
pub use embedded::EmbeddedClientConfig;
pub use execute::{ExecuteRequest, ExecuteResult, Tracing};
pub use hints::ZiskHints;
pub use input::ProgramInput;
pub use prove::{ProofHandle, ProofKind, ProveRequest, WatchEvent};
// pub use program::{Elf, GuestProgram, ProgramId};
pub use proof::Proof;
pub use remote::RemoteClientConfig;
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
// Re-export types from zisk_common (avoid glob io::* — it conflicts with our ZiskStdin wrapper)
pub use zisk_common::{
    PlonkVkey, ProofMode, ZiskProgramVK, ZiskProof, ZiskProofWithPublicValues, ZiskPublics, ZiskVK,
};

pub use zisk_build::*;

use anyhow::Result;
use std::sync::Arc;

impl<C: Client + Send + Sync> Client for Arc<C> {
    fn run_upload(&self, program: &GuestProgram) -> Result<()> {
        (**self).run_upload(program)
    }

    fn run_setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()> {
        (**self).run_setup(program, with_hints)
    }

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        cancel: Option<&CancellationToken>,
    ) -> Result<Proof> {
        (**self).run_prove(program, input, executor, mode, cancel)
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        cancel: Option<&CancellationToken>,
    ) -> Result<ExecuteResult> {
        (**self).run_execute(program, input, executor, cancel)
    }

    fn run_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        mode: ProofMode,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        (**self).run_wrap(proof_with_publics, mode, override_publics, override_program_vk)
    }
}

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

pub(crate) trait Client: Send + Sync {
    fn run_upload(&self, program: &GuestProgram) -> Result<()>;
    fn run_setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()>;
    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        cancel: Option<&CancellationToken>,
    ) -> Result<Proof>;
    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        cancel: Option<&CancellationToken>,
    ) -> Result<ExecuteResult>;
    fn run_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        mode: ProofMode,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues>;
}
