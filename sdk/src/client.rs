//! ZisK Prover Client
//!
//! This module provides an interface for interacting with the ZisK prover.

use crate::asm::builder::AsmProverBuilder;
use crate::emu::builder::EmuProverBuilder;

pub struct ProverClient;

impl ProverClient {
    #[must_use]
    pub fn builder() -> ProverClientBuilder {
        ProverClientBuilder
    }
}

/// A builder to define which proving client to use.
pub struct ProverClientBuilder;

impl ProverClientBuilder {
    #[must_use]
    pub fn emu(&self) -> EmuProverBuilder {
        EmuProverBuilder::new()
    }

    #[must_use]
    pub fn asm(&self) -> AsmProverBuilder {
        AsmProverBuilder::new()
    }
}
