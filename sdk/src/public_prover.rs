/// Public wrapper for ZiskProver that exposes only the SDK's public API
///
/// This wrapper provides access to the core proving functionality while hiding
/// advanced methods that should only be used by internal tools (CLI, distributed).
use anyhow::Result;
use zisk_common::io::ZiskStdin;
use zisk_common::{ZiskProgramVK, ZiskProofWithPublicValues};
use zisk_prover_backend::{
    Asm, AsmSetupBuilder, Emu, EmuSetupBuilder, GuestProgram, PlonkBuilder, ReduceBuilder,
    ZiskBackend, ZiskExecuteResult, ZiskProver,
};

pub struct PublicZiskProver<C: ZiskBackend> {
    pub(crate) inner: ZiskProver<C>,
}

impl<C: ZiskBackend> PublicZiskProver<C> {
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
        self.inner.execute(program, stdin)
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
        self.inner.prove(program, stdin)
    }

    /// Generate a PLONK/SNARK proof from an existing proof.
    /// Returns a `PlonkBuilder` that allows overriding publics or program_vk.
    ///
    /// # Example
    /// ```ignore
    /// let snark = prover.plonk(&proof_with_publics).run()?;
    /// ```
    pub fn plonk<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
    ) -> PlonkBuilder<'a, C> {
        self.inner.plonk(proof_with_publics)
    }

    /// Reduce a proof to a smaller, more efficient representation.
    /// Returns a `ReduceBuilder` that allows overriding publics or program_vk.
    ///
    /// # Example
    /// ```ignore
    /// let reduced = prover.reduce(&proof_with_publics).run()?;
    /// ```
    pub fn reduce<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
    ) -> ReduceBuilder<'a, C> {
        self.inner.reduce(proof_with_publics)
    }
}

// ASM-specific setup implementation
impl PublicZiskProver<Asm> {
    pub fn setup<'a>(&'a self, elf: &'a GuestProgram) -> AsmSetupBuilder<'a> {
        self.inner.setup(elf)
    }
}

// EMU-specific setup implementation
impl PublicZiskProver<Emu> {
    pub fn setup<'a>(&'a self, elf: &'a GuestProgram) -> EmuSetupBuilder<'a> {
        self.inner.setup(elf)
    }
}
