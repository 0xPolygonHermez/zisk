/// Public wrapper for ZiskProver that exposes only the SDK's public API
///
/// This wrapper provides access to the core proving functionality while hiding
/// advanced methods that should only be used by internal tools (CLI, distributed).
use anyhow::Result;
use zisk_common::{io::StreamSource, ZiskProgramVK};

use crate::ZiskStdin;
use zisk_prover_backend::{
    Asm, AsmSetupBuilder, Emu, EmuSetupBuilder, GuestProgram, ZiskBackend, ZiskExecuteResult,
    ZiskProver,
};

pub(crate) struct ZiskProverSDK<C: ZiskBackend> {
    pub(crate) inner: ZiskProver<C>,
}

impl<C: ZiskBackend> ZiskProverSDK<C> {
    pub(crate) fn new(prover: ZiskProver<C>) -> Self {
        Self { inner: prover }
    }

    /// Get the verification key for a guest program
    pub fn vk(&self, elf: &GuestProgram) -> Result<ZiskProgramVK> {
        self.inner.vk(elf)
    }

    /// Execute the program without generating a proof.
    /// The program must have been setup previously using `.setup()`.
    pub fn execute(&self, program: &GuestProgram, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        self.inner.execute(program, stdin.into_inner())
    }

    /// Generate a proof with the given standard input.
    /// Returns a `ProveBuilder` that allows setting per-proof options before running.
    /// The program must have been setup previously using `.setup()`.
    ///
    /// # Example
    /// ```ignore
    /// let result = prover.prove(&program, stdin).reduced().run()?;
    /// ```
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> zisk_prover_backend::ProveBuilder<'a, C> {
        self.inner.prove(program, stdin.into_inner())
    }

    pub(crate) fn register_hints_stream(&self, stream: StreamSource) -> Result<()> {
        self.inner.register_hints_stream(stream)
    }
}

// ASM-specific setup implementation
impl ZiskProverSDK<Asm> {
    pub fn setup<'a>(&'a self, elf: &'a GuestProgram) -> AsmSetupBuilder<'a> {
        self.inner.setup(elf)
    }
}

// EMU-specific setup implementation
impl ZiskProverSDK<Emu> {
    pub fn setup<'a>(&'a self, elf: &'a GuestProgram) -> EmuSetupBuilder<'a> {
        self.inner.setup(elf)
    }
}
