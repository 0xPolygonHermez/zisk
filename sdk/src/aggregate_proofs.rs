//! `client.aggregate_proofs(...)` builder.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use zisk_common::Proof;

use crate::job_handle::{subscriber_list_from, JobHandle, Subscriber};
use crate::prove::{JobEvent, ProveResult};
use crate::recurser::Recurser;
use crate::Client;

/// A proof entering a fold, optionally carrying the side inputs its
/// normalization circuit consumes. A plain `&Proof` converts with no
/// inputs — right for aggregated proofs and for leaves of ungrouped
/// programs; leaves whose group declares free inputs are paired with
/// them via [`ProofExt::with_free_inputs`].
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
    /// Pair this proof with the side inputs its normalization circuit
    /// consumes the first time it enters the recursion.
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
        // Each side must supply exactly what its proof's normalization group
        // consumes: classifying by programVK catches a forgotten
        // `with_free_inputs` here, instead of as a wrong digest (or a failed
        // constraint) deep inside witness generation.
        let max_n = self.agg.n_free_inputs();
        let check_and_pad = |side: char, input: &AggregationInput<'_>| -> Result<Vec<u64>> {
            let expected = expected_free_inputs(self.agg, input.proof);
            let got = input.free_inputs.len();
            if got != expected {
                return Err(anyhow!(
                    "proof_{side} supplies {got} free inputs but its normalization group \
                     consumes {expected}{}",
                    if expected == 0 {
                        " (aggregated proofs and ungrouped leaves take none — pass a plain \
                         `&Proof`)"
                    } else {
                        ""
                    },
                ));
            }
            // The circuit's per-side arrays are fixed at the worst case across
            // groups; the unused tail is zero.
            let mut v = input.free_inputs.clone();
            v.resize(max_n, 0);
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

/// How many free inputs this proof's normalization consumes: its group's
/// count when it's a leaf of a grouped program, zero for ungrouped leaves
/// and aggregated proofs (whose programVK is not in the allowlist).
fn expected_free_inputs(agg: &Recurser, proof: &Proof) -> usize {
    let vk = &proof.get_program_vk().vk;
    let Some(program_idx) = agg
        .program_vks
        .iter()
        .position(|p| p.len() == vk.len() && p.iter().zip(vk).all(|(s, w)| s == &w.to_string()))
    else {
        return 0;
    };
    agg.templates
        .normalize_groups
        .iter()
        .find(|g| g.member_indices.contains(&program_idx))
        .map(|g| g.n_free_inputs)
        .unwrap_or(0)
}
