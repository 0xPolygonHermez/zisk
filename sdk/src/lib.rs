mod async_prove;
mod client;
pub(crate) mod core;
mod embedded;
mod execute;
mod hints;
mod input;
mod plonk;
mod proof;
mod prove;
mod reduce;
mod setup;
mod stdin;
mod upload;

pub use async_prove::{AsyncProveRequest, ProofHandle};
pub use client::ProverClient;
pub use embedded::{EmbeddedClientBuilder, EmbeddedOptions};
pub use execute::{ExecuteRequest, ExecuteResult, Tracing};
pub use hints::ZiskHints;
pub use input::ProgramInput;
// pub use program::{Elf, GuestProgram, ProgramId};
pub use plonk::PlonkRequest;
pub use proof::Proof;
pub use prove::{ProofKind, ProveRequest, WatchEvent};
pub use reduce::ReduceRequest;
pub use setup::SetupRequest;
pub use stdin::ZiskStdin;
pub use upload::UploadRequest;

// Re-export guest types from backend (public API for loading programs)
pub use zisk_prover_backend::{load_program, Elf, EmuOptions, GuestProgram, ProgramId};

// Re-export result and data types from backend (public outputs)
pub use zisk_prover_backend::{
    PlonkBuilder, ProofOpts, ProveBuilder, ReduceBuilder, ZiskExecuteResult, ZiskProveResult,
    ZiskVerifyConstraintsResult,
};

// Re-export common types
pub use proofman_common::VerboseMode;

// Re-export types from zisk_common
pub use zisk_common::{
    io::*, PlonkVkey, ProofMode, ZiskProgramVK, ZiskProof, ZiskProofWithPublicValues, ZiskPublics,
    ZiskVK,
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
        opts: ProofOpts,
    ) -> Result<Proof> {
        (**self).run_prove(program, input, executor, mode, opts)
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
    ) -> Result<ExecuteResult> {
        (**self).run_execute(program, input, executor)
    }

    fn run_reduce(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        (**self).run_reduce(proof_with_publics, override_publics, override_program_vk)
    }

    fn run_plonk(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        (**self).run_plonk(proof_with_publics, override_publics, override_program_vk)
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

/// Core client trait implemented by all prover backends.
pub trait Client: Send + Sync {
    /// Run an upload operation for the given program.
    fn run_upload(&self, program: &GuestProgram) -> Result<()>;

    /// Run a ROM setup for the given program.
    fn run_setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()>;

    /// Run a prove operation with the given executor.
    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        opts: ProofOpts,
    ) -> Result<Proof>;

    /// Run an execute operation (dry-run, no proof) with the given executor.
    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
    ) -> Result<ExecuteResult>;

    /// Reduce a full STARK proof to a compressed form.
    fn run_reduce(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues>;

    /// Wrap a full STARK proof into a PLONK/SNARK proof.
    fn run_plonk(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues>;
}
