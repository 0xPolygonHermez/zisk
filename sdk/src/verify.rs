use crate::{Result, SdkError};
use zisk_common::{PlonkVkey, ProgramVK, Proof, PublicValues};

/// Builder for proof verification with externally-supplied overrides.
pub struct VerifyBuilder<'a> {
    proof: &'a Proof,
    publics: Option<&'a PublicValues>,
    program_vk: Option<&'a ProgramVK>,
    plonk_vk: Option<&'a PlonkVkey>,
    setup_vk: Option<&'a [u64]>,
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

    /// Optional trusted PLONK circuit key; if unset, the proof's embedded key is used.
    #[must_use]
    pub fn with_plonk_vk(mut self, vkey: &'a PlonkVkey) -> Self {
        self.plonk_vk = Some(vkey);
        self
    }

    /// Optional trusted recursion setup key (4 u64 limbs); if unset, the proof's
    /// embedded key is used.
    #[must_use]
    pub fn with_setup_vk(mut self, setup_vk: &'a [u64]) -> Self {
        self.setup_vk = Some(setup_vk);
        self
    }

    /// Run the verification.
    pub fn verify(self) -> Result<()> {
        let mut builder = self.proof.verify_builder();
        if let Some(p) = self.publics {
            builder = builder.with_publics(p);
        }
        if let Some(v) = self.program_vk {
            builder = builder.with_program_vk(v);
        }
        if let Some(k) = self.plonk_vk {
            builder = builder.with_plonk_vk(k);
        }
        if let Some(k) = self.setup_vk {
            builder = builder.with_setup_vk(k);
        }
        builder.verify().map_err(SdkError::backend)
    }
}
