use anyhow::Result;

use crate::{Client, ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};

/// Builder for a minimal proof request.
///
/// Obtain via `client.minimal(&proof_with_publics)`.
///
/// Reduces a full STARK proof (`ZiskProof::VadcopFinal`) to a minimal form
/// (`ZiskProof::VadcopFinalMinimal`), which is smaller and faster to verify.
#[allow(dead_code)]
pub struct MinimalRequest<'a, C> {
    client: &'a C,
    proof_with_publics: &'a ZiskProofWithPublicValues,
    override_publics: Option<&'a ZiskPublics>,
    override_program_vk: Option<&'a ZiskProgramVK>,
}

#[allow(private_bounds)]
impl<'a, C: Client> MinimalRequest<'a, C> {
    pub(crate) fn new(client: &'a C, proof_with_publics: &'a ZiskProofWithPublicValues) -> Self {
        Self { client, proof_with_publics, override_publics: None, override_program_vk: None }
    }

    /// Override the public inputs used during reduction.
    #[must_use]
    pub fn with_publics(mut self, publics: &'a ZiskPublics) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key used during reduction.
    #[must_use]
    pub fn with_program_vk(mut self, program_vk: &'a ZiskProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Run the reduction.
    pub fn run(self) -> Result<ZiskProofWithPublicValues> {
        self.client.run_minimal(
            self.proof_with_publics,
            self.override_publics,
            self.override_program_vk,
        )
    }
}
