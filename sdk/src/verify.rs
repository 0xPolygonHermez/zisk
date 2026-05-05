use anyhow::Result;
use zisk_common::{ProgramVK, Proof, PublicValues};

/// Builder for proof verification with externally-supplied overrides.
pub struct VerifyBuilder<'a> {
    proof: &'a Proof,
    publics: Option<&'a PublicValues>,
    program_vk: Option<&'a ProgramVK>,
}

impl<'a> VerifyBuilder<'a> {
    /// Override the public values embedded in the proof.
    #[must_use]
    pub fn with_publics(mut self, pv: &'a PublicValues) -> Self {
        self.publics = Some(pv);
        self
    }

    /// Override the verification key embedded in the proof.
    #[must_use]
    pub fn with_program_vk(mut self, vk: &'a ProgramVK) -> Self {
        self.program_vk = Some(vk);
        self
    }

    /// Run the verification.
    pub fn verify(self) -> Result<()> {
        match (self.publics, self.program_vk) {
            (None, None) => self.proof.verify(),
            (Some(p), None) => self.proof.with_publics(p).verify(),
            (None, Some(v)) => self.proof.with_program_vk(v).verify(),
            (Some(p), Some(v)) => self.proof.with_publics(p).with_program_vk(v).verify(),
        }
    }
}
