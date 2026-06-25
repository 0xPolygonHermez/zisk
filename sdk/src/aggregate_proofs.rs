//! `client.aggregate_proofs(...)` builder.

use std::sync::Arc;
use std::time::Duration;

use zisk_common::Proof;

use crate::job_handle::{subscriber_list_from, JobHandle, Subscriber};
use crate::prove::{JobEvent, ProveResult};
use crate::recurser::Recurser;
use crate::{Client, Result, SdkError};

/// A proof entering a fold, optionally carrying the side inputs the
/// `AggregatePublics` circuit reads. A plain `&Proof` converts with no
/// inputs — right when the recurser declares no aggregate free inputs;
/// otherwise pair each side with its inputs via
/// [`ProofExt::with_free_inputs`].
pub struct AggregationInput<'a> {
    pub(crate) proof: &'a Proof,
    pub(crate) free_inputs: Vec<u64>,
}

impl<'a> From<&'a Proof> for AggregationInput<'a> {
    fn from(proof: &'a Proof) -> Self {
        Self { proof, free_inputs: Vec::new() }
    }
}

/// Sugar for building an [`AggregationInput`] from a [`Proof`].
pub trait ProofExt {
    /// Pair this proof with the side inputs the `AggregatePublics` circuit
    /// reads for this side of the fold.
    fn with_free_inputs(&self, free_inputs: impl Into<Vec<u64>>) -> AggregationInput<'_>;
}

impl ProofExt for Proof {
    fn with_free_inputs(&self, free_inputs: impl Into<Vec<u64>>) -> AggregationInput<'_> {
        AggregationInput { proof: self, free_inputs: free_inputs.into() }
    }
}

/// Builder for a recurser prove request. Obtain via
/// `client.aggregate_proofs(&agg, &proof_a, &proof_b)` — each side accepts
/// a `&Proof` or a [`ProofExt::with_free_inputs`] pairing.
pub struct AggregateProofsRequest<'a, C> {
    client: &'a C,
    agg: &'a Recurser,
    input_a: AggregationInput<'a>,
    input_b: AggregationInput<'a>,
    root_c_recurser_agg: Option<[u64; 4]>,
    timeout: Option<Duration>,
    subscribers: Vec<Subscriber>,
}

#[allow(private_bounds)]
impl<'a, C: Client> AggregateProofsRequest<'a, C> {
    pub(crate) fn new(
        client: &'a C,
        agg: &'a Recurser,
        input_a: AggregationInput<'a>,
        input_b: AggregationInput<'a>,
    ) -> Self {
        Self {
            client,
            agg,
            input_a,
            input_b,
            root_c_recurser_agg: None,
            timeout: None,
            subscribers: Vec::new(),
        }
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
        // Each side must supply exactly the free inputs the `AggregatePublics`
        // circuit reads. Checking here catches a forgotten `with_free_inputs`
        // up front, instead of as a wrong digest (or a failed constraint) deep
        // inside witness generation.
        let expected = self.agg.n_free_inputs();
        let check_and_pad = |side: char, input: &AggregationInput<'_>| -> Result<Vec<u64>> {
            let got = input.free_inputs.len();
            if got != expected {
                return Err(SdkError::Recurser(format!(
                    "proof_{side} supplies {got} free inputs but the aggregate circuit \
                     consumes {expected}{}",
                    if expected == 0 { " (pass a plain `&Proof`)" } else { "" },
                )));
            }
            let mut v = input.free_inputs.clone();
            v.resize(expected, 0);
            Ok(v)
        };
        let free_inputs_a = check_and_pad('a', &self.input_a)?;
        let free_inputs_b = check_and_pad('b', &self.input_b)?;

        let subs = subscriber_list_from(self.subscribers);
        self.client.run_aggregate_proofs(
            self.agg,
            self.input_a.proof,
            self.input_b.proof,
            &free_inputs_a,
            &free_inputs_b,
            self.root_c_recurser_agg,
            self.timeout,
            subs,
        )
    }
}
