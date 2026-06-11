//! `client.recurser_prove(...)` builder.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use zisk_common::Proof;

use crate::job_handle::{subscriber_list_from, JobHandle, Subscriber};
use crate::prove::{JobEvent, ProveResult};
use crate::recurser::Recurser;
use crate::Client;

/// Builder for a recurser prove request. Obtain via
/// `client.recurser_prove(&agg, &proof_a, &proof_b)`.
pub struct RecurserProveRequest<'a, C> {
    client: &'a C,
    agg: &'a Recurser,
    proof_a: &'a Proof,
    proof_b: &'a Proof,
    private_inputs: Vec<u64>,
    root_c_recurser_agg: Option<[u64; 4]>,
    timeout: Option<Duration>,
    subscribers: Vec<Subscriber>,
}

#[allow(private_bounds)]
impl<'a, C: Client> RecurserProveRequest<'a, C> {
    pub(crate) fn new(
        client: &'a C,
        agg: &'a Recurser,
        proof_a: &'a Proof,
        proof_b: &'a Proof,
    ) -> Self {
        Self {
            client,
            agg,
            proof_a,
            proof_b,
            private_inputs: Vec::new(),
            root_c_recurser_agg: None,
            timeout: None,
            subscribers: Vec::new(),
        }
    }

    /// Private inputs threaded into the recurser's templates.
    #[must_use]
    pub fn private_inputs(mut self, v: impl Into<Vec<u64>>) -> Self {
        self.private_inputs = v.into();
        self
    }

    /// Override `rootCRecurserAgg`. By default reads the recurser's own verkey.
    #[must_use]
    pub fn root_c_recurser_agg(mut self, limbs: [u64; 4]) -> Self {
        self.root_c_recurser_agg = Some(limbs);
        self
    }

    /// Set a timeout for proof generation.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Register a pre-submit event callback.
    #[must_use]
    pub fn on(mut self, event: JobEvent, cb: impl Fn(JobEvent) + Send + Sync + 'static) -> Self {
        self.subscribers.push((event, Arc::new(cb)));
        self
    }

    /// Submit the recurser prove, returning a [`JobHandle<ProveResult>`].
    pub fn run(self) -> Result<JobHandle<ProveResult>> {
        if self.private_inputs.len() != self.agg.n_private_inputs {
            return Err(anyhow!(
                "private_inputs length {} does not match recurser's n_private_inputs {}",
                self.private_inputs.len(),
                self.agg.n_private_inputs
            ));
        }
        let subs = subscriber_list_from(self.subscribers);
        self.client.run_recurser_prove(
            self.agg,
            self.proof_a,
            self.proof_b,
            &self.private_inputs,
            self.root_c_recurser_agg,
            self.timeout,
            subs,
        )
    }
}
