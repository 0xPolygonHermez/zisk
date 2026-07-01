use std::sync::atomic::{AtomicBool, Ordering};

use crate::{embedded::EmbeddedClientBuilder, remote::RemoteClientBuilder};

static PROVER_CLIENT_CREATED: AtomicBool = AtomicBool::new(false);

pub(crate) fn ensure_single_instance() {
    if PROVER_CLIENT_CREATED.swap(true, Ordering::AcqRel) {
        panic!(
            "A ProverClient already exists. Only one instance is allowed per process. \
             Store it in a shared location (e.g., Arc<EmbeddedClient> / Arc<RemoteClient>) and reuse it."
        );
    }
}

/// Entry-point namespace for building **concrete, fully-typed** prover clients.
///
/// Obtain a client via:
/// - `ProverClient::embedded().build()` → [`EmbeddedClient`](crate::EmbeddedClient)
/// - `ProverClient::remote(url).build()` → [`RemoteClient`](crate::RemoteClient)
///
/// The returned clients carry their full backend-specific surface (the embedded client's
/// synchronous `.run_sync()` and `verify_constraints`; the remote client's `setup_by_id`).
///
/// When the backend is chosen at runtime, use [`ZiskClient`](crate::ZiskClient) instead — its
/// `embedded()`/`remote()` builders `build()` into a single runtime-dispatch type.
pub struct ProverClient;

impl ProverClient {
    /// Returns a builder for the embedded (local) backend.
    #[must_use]
    pub fn embedded() -> EmbeddedClientBuilder {
        EmbeddedClientBuilder::default()
    }

    /// Returns a builder for the remote (coordinator) backend.
    ///
    /// # Example
    /// ```ignore
    /// let client = ProverClient::remote("http://coordinator:50051").build()?;
    /// ```
    #[must_use]
    pub fn remote(url: impl Into<String>) -> RemoteClientBuilder {
        RemoteClientBuilder::new(url)
    }
}
