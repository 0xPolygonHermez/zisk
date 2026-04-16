use anyhow::Result;
use std::time::Duration;
use zisk_common::{StatsCostPerType, ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};
use zisk_prover_backend::ZiskProveResult;

/// A completed ZisK proof with its public values.
pub struct Proof {
    pub(crate) inner: ZiskProveResult,
}

impl Proof {
    pub(crate) fn new(inner: ZiskProveResult) -> Self {
        Self { inner }
    }

    pub fn get_duration(&self) -> Duration {
        self.inner.get_duration()
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.inner.get_execution_steps()
    }

    pub fn get_execution_total_cost(&self) -> u64 {
        self.inner.get_execution_total_cost()
    }

    pub fn get_execution_cost_per_type(&self) -> &StatsCostPerType {
        self.inner.get_execution_cost_per_type()
    }

    pub fn get_proof(&self) -> &ZiskProofWithPublicValues {
        self.inner.get_proof()
    }

    pub fn get_proof_bytes(&self) -> Vec<u8> {
        self.inner.get_proof_bytes()
    }

    pub fn save_proof(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        self.inner.save_proof(path)
    }

    pub fn get_public_values(&self) -> &[u8] {
        self.inner.get_public_values()
    }

    /// Verify the proof using the public values and verification key embedded in the proof.
    pub fn verify(&self) -> Result<()> {
        self.inner.verify()
    }

    /// Override the verification key for this verification.
    pub fn with_program_vk<'a>(&'a self, vk: &'a ZiskProgramVK) -> VerifyBuilder<'a> {
        VerifyBuilder { proof: self, publics: None, program_vk: Some(vk) }
    }

    /// Override the public values for this verification.
    pub fn with_publics<'a>(&'a self, pv: &'a ZiskPublics) -> VerifyBuilder<'a> {
        VerifyBuilder { proof: self, publics: Some(pv), program_vk: None }
    }
}

/// Builder for proof verification with externally-supplied overrides.
pub struct VerifyBuilder<'a> {
    proof: &'a Proof,
    publics: Option<&'a ZiskPublics>,
    program_vk: Option<&'a ZiskProgramVK>,
}

impl<'a> VerifyBuilder<'a> {
    /// Override the public values embedded in the proof.
    #[must_use]
    pub fn with_publics(mut self, pv: &'a ZiskPublics) -> Self {
        self.publics = Some(pv);
        self
    }

    /// Override the verification key embedded in the proof.
    #[must_use]
    pub fn with_program_vk(mut self, vk: &'a ZiskProgramVK) -> Self {
        self.program_vk = Some(vk);
        self
    }

    /// Run the verification.
    pub fn verify(self) -> Result<()> {
        match (self.publics, self.program_vk) {
            (None, None) => self.proof.inner.verify(),
            (Some(p), None) => self.proof.inner.with_publics(p).verify(),
            (None, Some(v)) => self.proof.inner.with_program_vk(v).verify(),
            (Some(p), Some(v)) => self.proof.inner.with_publics(p).with_program_vk(v).verify(),
        }
    }
}
