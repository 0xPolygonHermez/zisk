use anyhow::Result;
use zisk_common::{ZiskProgramVK, ZiskPublics};
use zisk_prover_backend::ZiskProveResult;

/// A completed ZisK proof with its public values.
pub struct Proof {
    pub(crate) inner: ZiskProveResult,
}

impl Proof {
    pub(crate) fn new(inner: ZiskProveResult) -> Self {
        Self { inner }
    }

    /// Verify the proof using the public values and verification key embedded in the proof.
    pub fn verify(&self) -> Result<()> {
        self.inner.verify()
    }

    /// Override the verification key for this verification.
    pub fn verification_key<'a>(&'a self, vk: &'a ZiskProgramVK) -> VerifyBuilder<'a> {
        VerifyBuilder { proof: self, publics: None, vk: Some(vk) }
    }

    /// Override the public values for this verification.
    pub fn publics<'a>(&'a self, pv: &'a ZiskPublics) -> VerifyBuilder<'a> {
        VerifyBuilder { proof: self, publics: Some(pv), vk: None }
    }
}

/// Builder for proof verification with externally-supplied overrides.
pub struct VerifyBuilder<'a> {
    proof: &'a Proof,
    publics: Option<&'a ZiskPublics>,
    vk: Option<&'a ZiskProgramVK>,
}

impl<'a> VerifyBuilder<'a> {
    /// Override the public values embedded in the proof.
    #[must_use]
    pub fn publics(mut self, pv: &'a ZiskPublics) -> Self {
        self.publics = Some(pv);
        self
    }

    /// Override the verification key embedded in the proof.
    #[must_use]
    pub fn verification_key(mut self, vk: &'a ZiskProgramVK) -> Self {
        self.vk = Some(vk);
        self
    }

    /// Run the verification.
    pub fn verify(self) -> Result<()> {
        match (self.publics, self.vk) {
            (None, None) => self.proof.inner.verify(),
            (Some(p), None) => self.proof.inner.publics(p).verify(),
            (None, Some(v)) => self.proof.inner.program_vk(v).verify(),
            (Some(p), Some(v)) => self.proof.inner.publics(p).program_vk(v).verify(),
        }
    }
}
