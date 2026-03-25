/// Public wrapper for ZiskProver that exposes only the SDK's public API
///
/// This wrapper provides access to the core proving functionality while hiding
/// advanced methods that should only be used by internal tools (CLI, distributed).
use anyhow::Result;
use zisk_common::io::ZiskStdin;
use zisk_common::{ZiskProgramVK, ZiskProofWithPublicValues};
use zisk_prover_backend::{
    GuestProgram, PlonkBuilder, ProveBuilder, ReduceBuilder, ZiskBackend, ZiskExecuteResult,
    ZiskProgramPK, ZiskProver,
};

pub struct PublicZiskProver<C: ZiskBackend> {
    inner: ZiskProver<C>,
}

impl<C: ZiskBackend> PublicZiskProver<C> {
    pub(crate) fn new(prover: ZiskProver<C>) -> Self {
        Self { inner: prover }
    }

    /// Setup a guest program and return the proving key and verification key
    pub fn setup(
        &self,
        elf: &GuestProgram,
        with_hints: bool,
    ) -> Result<(ZiskProgramPK, ZiskProgramVK)> {
        self.inner.setup(elf, with_hints)
    }

    /// Get the verification key for a guest program
    pub fn vk(&self, elf: &GuestProgram) -> Result<ZiskProgramVK> {
        self.inner.vk(elf)
    }

    /// Execute the program without generating a proof
    pub fn execute(&self, pk: &ZiskProgramPK, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        self.inner.execute(pk, stdin)
    }

    /// Generate a proof with the given standard input.
    /// Returns a `ProveBuilder` that allows setting per-proof options before running.
    ///
    /// # Example
    /// ```ignore
    /// let result = prover.prove(&pk, stdin).reduced().run()?;
    /// ```
    pub fn prove<'a>(&'a self, pk: &'a ZiskProgramPK, stdin: ZiskStdin) -> ProveBuilder<'a, C> {
        self.inner.prove(pk, stdin)
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
