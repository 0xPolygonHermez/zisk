//! `client.aggregate_proofs(...)` builder.

use std::sync::Arc;
use std::time::Duration;

use zisk_common::Proof;

use crate::job_handle::{subscriber_list_from, JobHandle, Subscriber};
use crate::prove::{JobEvent, ProveResult};
use crate::recurser::Recurser;
use crate::{Client, Result, SdkError};

/// A proof entering a fold, optionally carrying its per-side free array.
///
/// A plain `&Proof` converts with an empty array — correct when the recurser
/// declares no free values. Attach the single free array via
/// [`ProofExt::with_free_inputs`]:
///
/// ```ignore
/// proof.with_free_inputs(free_vals)
/// ```
///
/// The array has width `recurser.n_free()` per side. On a leaf proof it is the
/// free_in (normalized internally); on an aggregated proof it is the free_out
/// (used directly). Either way it is one array of width `n_free`.
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
    /// Pair this proof with its per-side free array (width `recurser.n_free()`).
    fn with_free_inputs(&self, inputs: impl Into<Vec<u64>>) -> AggregationInput<'_>;
}

impl ProofExt for Proof {
    fn with_free_inputs(&self, inputs: impl Into<Vec<u64>>) -> AggregationInput<'_> {
        AggregationInput { proof: self, free_inputs: inputs.into() }
    }
}

/// Builder for a recurser prove request. Obtain via
/// `client.aggregate_proofs(&agg, &proof_a, &proof_b)` — each side accepts
/// a `&Proof` or a [`ProofExt`] pairing.
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
    ///
    /// # Free-input layout
    ///
    /// One free array per side (width exactly `recurser.n_free()`) is passed
    /// straight through to the backend. Per-side semantics: on a leaf it is the
    /// free_in (normalized internally into free_out); on an aggregated proof it
    /// is the free_out (used directly). The array width is the same either way.
    /// A plain `&Proof` converts with an empty array — valid only when
    /// `n_free == 0`.
    pub fn run(self) -> Result<JobHandle<ProveResult>> {
        // Per-side classification (leaf vs aggregated) is by
        // publics_full()[IS_VADCOP_FINAL_SLOT]: 1 = leaf (array is free_in,
        // normalized into free_out), 0 = aggregated (array is free_out, used
        // directly). The width is the same either way, so the SDK does not
        // branch on it here; the backend's validate_prove_inputs is the real
        // gate. Each side must carry exactly n_free values — the backend fills
        // the witness buffer positionally with no padding, so any other length
        // shears rootCRecurserAgg out of place.
        let n_free = self.agg.n_free();

        let validate = |side: char, input: &AggregationInput<'_>| -> Result<()> {
            let got = input.free_inputs.len();
            if got != n_free {
                return Err(SdkError::Recurser(format!(
                    "proof_{side} supplies {got} free inputs but the recurser \
                     consumes exactly {n_free} (a plain &Proof supplies 0; use \
                     `.with_free_inputs(..)` to attach exactly {n_free})",
                )));
            }
            Ok(())
        };

        validate('a', &self.input_a)?;
        validate('b', &self.input_b)?;

        let free_a = self.input_a.free_inputs;
        let free_b = self.input_b.free_inputs;

        let subs = subscriber_list_from(self.subscribers);
        self.client.run_aggregate_proofs(
            self.agg,
            self.input_a.proof,
            self.input_b.proof,
            &free_a,
            &free_b,
            self.root_c_recurser_agg,
            self.timeout,
            subs,
        )
    }
}
