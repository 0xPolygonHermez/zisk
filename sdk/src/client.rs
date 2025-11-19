//! ZisK Prover Client
//!
//! This module provides an interface for interacting with the ZisK prover.
//!
//! For new code, prefer using `ZiskProverBuilder` directly for type-safe configuration.
//! This legacy interface is maintained for backward compatibility.

use crate::ProverClientBuilder;

pub struct ProverClient;

impl ProverClient {
    #[must_use]
    pub fn builder() -> ProverClientBuilder {
        ProverClientBuilder::new()
    }
}
