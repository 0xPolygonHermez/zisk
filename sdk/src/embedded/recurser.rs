//! Embedded dispatch for recurser operations.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use recurser::setup::{run_setup_recurser_aggregator, SetupRecurserAggregatorOptions};
use zisk_common::{Proof, StatsCostPerType};
use zisk_prover_backend::ProveOutput;

use super::{EmbeddedClient, EmbeddedProver};
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::prove::ProveResult;
use crate::recurser::Recurser;
use crate::setup::SetupResult;
use crate::upload::UploadResult;
use crate::JobEvent;

impl EmbeddedClient {
    pub(crate) fn do_upload_aggregation_program(&self, agg: &Recurser) -> Result<UploadResult> {
        Ok(UploadResult::new(agg.recurser_id.clone()))
    }

    pub(crate) fn do_setup_aggregation_program(
        &self,
        agg: &Recurser,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let agg = agg.clone();
        let subs_cloned = Arc::clone(&subs);
        let prover = Arc::clone(&self.prover);

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);
            let result = run_setup_aggregation_program_blocking(&prover, &agg);
            fire_result_event(&subs_cloned, &result);
            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    pub(crate) fn do_setup_aggregation_program_sync(
        &self,
        agg: &Recurser,
        subs: SubscriberList,
    ) -> Result<SetupResult> {
        fire_event(&subs, JobEvent::Started);
        let result = run_setup_aggregation_program_blocking(&self.prover, agg);
        fire_result_event(&subs, &result);
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_aggregate_proofs(
        &self,
        agg: &Recurser,
        proof_a: &Proof,
        proof_b: &Proof,
        free_inputs_a: &[u64],
        free_inputs_b: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        let agg = agg.clone();
        let vfp_a = proof_a.get_vadcop_final_proof()?;
        let vfp_b = proof_b.get_vadcop_final_proof()?;
        let free_inputs_a = free_inputs_a.to_vec();
        let free_inputs_b = free_inputs_b.to_vec();
        let subs_cloned = Arc::clone(&subs);
        let prover = Arc::clone(&self.prover);

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);
            let result = run_aggregate_proofs_blocking(
                &prover,
                &agg,
                vfp_a,
                vfp_b,
                &free_inputs_a,
                &free_inputs_b,
                root_c_recurser_agg,
            );
            fire_result_event(&subs_cloned, &result);
            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }
}

fn run_setup_aggregation_program_blocking(
    prover: &EmbeddedProver,
    agg: &Recurser,
) -> Result<SetupResult> {
    let artifacts = recurser::artifacts::RecurserArtifacts::new(&agg.output_dir, &agg.recurser_id);
    if artifacts.is_active() {
        tracing::info!(
            "Recurser '{}' already set up at {}; registering",
            agg.recurser_id,
            artifacts.dir().display()
        );
        // Artifacts are on disk but proofman in this process still needs to
        // register them before proving.
        prover
            .register_recurser(&agg.output_dir, &agg.recurser_id)
            .context("recurser registration failed")?;
        return Ok(SetupResult { job_id: None });
    }

    let opts = SetupRecurserAggregatorOptions {
        setup_dir: agg.setup_dir.clone(),
        output_dir: agg.output_dir.clone(),
        program_vks: agg.program_vks.clone(),
        templates: agg.templates.clone(),
    };

    // Scoped 64 MB-stack rayon pool — proofman setup overflows the default
    // ~2 MB worker stack on Circom / STARK preprocessing. Prove doesn't need
    // this; it runs in FFI.
    let pool = rayon::ThreadPoolBuilder::new()
        .stack_size(64 * 1024 * 1024)
        .build()
        .map_err(|e| anyhow!("failed to build 64 MB-stack rayon pool: {e}"))?;
    pool.install(|| run_setup_recurser_aggregator(&opts)).context("recurser setup failed")?;

    // Register the freshly-generated setup with proofman so it can prove.
    prover
        .register_recurser(&agg.output_dir, &agg.recurser_id)
        .context("recurser registration failed")?;
    Ok(SetupResult { job_id: None })
}

fn run_aggregate_proofs_blocking(
    prover: &EmbeddedProver,
    agg: &Recurser,
    proof_a: proofman_verifier::VadcopFinalProof,
    proof_b: proofman_verifier::VadcopFinalProof,
    free_inputs_a: &[u64],
    free_inputs_b: &[u64],
    root_c_override: Option<[u64; 4]>,
) -> Result<ProveResult> {
    let vfp = prover
        .prove_recurser(
            &agg.recurser_id,
            &proof_a,
            &proof_b,
            free_inputs_a,
            free_inputs_b,
            root_c_override,
        )
        .context("recurser proof generation failed")?;

    // Recurser's own verkey → output Proof's zisk_vk.
    let zisk_vk = agg.vk()?.vk;
    // The proof's hash family travels on the VadcopFinalProof (stamped by
    // proofman from the recurser's proving key); carry it onto the Proof.
    let proof = Proof::new_from_vadcop_proof(
        &vfp.proof_with_publics(),
        vfp.compressed,
        zisk_vk,
        vfp.hash.clone(),
    )?;

    Ok(ProveResult::from(ProveOutput::from_remote(
        proof,
        0,
        Duration::from_secs(0),
        StatsCostPerType::default(),
    )))
}
