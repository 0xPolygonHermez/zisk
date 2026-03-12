//! ZisK Prover Client
//!
//! This module provides an interface for interacting with the ZisK prover.
//!
//! For new code, prefer using `ZiskProverBuilder` directly for type-safe configuration.
//! This legacy interface is maintained for backward compatibility.

use crate::ProverClientBuilder;
use std::sync::atomic::{AtomicBool, Ordering};

static PROVER_CLIENT_CREATED: AtomicBool = AtomicBool::new(false);

pub struct ProverClient;

impl ProverClient {
    #[must_use]
    pub fn builder() -> ProverClientBuilder {
        if PROVER_CLIENT_CREATED.swap(true, Ordering::SeqCst) {
            panic!(
                "ProverClient::builder() can only be called once! \
                Multiple ProverClient instances are not supported. \
                Reuse the existing client for all operations."
            );
        }
        ProverClientBuilder::new()
    }
}
