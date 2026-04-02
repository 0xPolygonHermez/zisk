use anyhow::Result;

use crate::{Client, ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};

/// Builder for a PLONK/SNARK proof generation request.
///
/// Obtain via `client.plonk(&proof_with_publics)`.
///
/// Wraps a full STARK proof (`ZiskProof::VadcopFinal`) into a PLONK/SNARK proof
/// (`ZiskProof::Plonk`). Requires `snark_wrapper` to be initialised in the prover.
#[allow(dead_code)]
pub struct PlonkRequest<'a, C> {
    client: &'a C,
    proof_with_publics: &'a ZiskProofWithPublicValues,
    override_publics: Option<&'a ZiskPublics>,
    override_program_vk: Option<&'a ZiskProgramVK>,
}

#[allow(private_bounds)]
impl<'a, C: Client> PlonkRequest<'a, C> {
    pub(crate) fn new(client: &'a C, proof_with_publics: &'a ZiskProofWithPublicValues) -> Self {
        Self { client, proof_with_publics, override_publics: None, override_program_vk: None }
    }

    /// Override the public inputs used during PLONK proof generation.
    #[must_use]
    pub fn publics(mut self, publics: &'a ZiskPublics) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key used during PLONK proof generation.
    #[must_use]
    pub fn program_vk(mut self, program_vk: &'a ZiskProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Run the PLONK proof generation.
    pub fn run(self) -> Result<ZiskProofWithPublicValues> {
        self.client.run_plonk(
            self.proof_with_publics,
            self.override_publics,
            self.override_program_vk,
        )
    }
}
