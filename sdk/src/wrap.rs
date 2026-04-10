use anyhow::Result;

use crate::{Client, ProofMode, ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};

/// Builder for a proof wrapping/conversion request.
///
/// Obtain via `client.wrap(&proof_with_publics, mode)`.
///
/// Wraps or reduces a proof to a different format based on the `ProofMode`:
/// - `ProofMode::VadcopFinalMinimal`: Reduces a full STARK proof to a minimal form
/// - `ProofMode::Plonk`: Wraps a STARK proof into a PLONK/SNARK proof
#[allow(dead_code)]
pub struct WrapRequest<'a, C> {
    client: &'a C,
    proof_with_publics: &'a ZiskProofWithPublicValues,
    mode: ProofMode,
    override_publics: Option<&'a ZiskPublics>,
    override_program_vk: Option<&'a ZiskProgramVK>,
}

#[allow(private_bounds)]
impl<'a, C: Client> WrapRequest<'a, C> {
    pub(crate) fn new(
        client: &'a C,
        proof_with_publics: &'a ZiskProofWithPublicValues,
        mode: ProofMode,
    ) -> Self {
        Self { client, proof_with_publics, mode, override_publics: None, override_program_vk: None }
    }

    /// Override the public inputs used during wrapping.
    #[must_use]
    pub fn with_publics(mut self, publics: &'a ZiskPublics) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key used during wrapping.
    #[must_use]
    pub fn with_program_vk(mut self, program_vk: &'a ZiskProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Run the proof wrapping/conversion.
    pub fn run(self) -> Result<ZiskProofWithPublicValues> {
        self.client.run_wrap(
            self.proof_with_publics,
            self.mode,
            self.override_publics,
            self.override_program_vk,
        )
    }
}
