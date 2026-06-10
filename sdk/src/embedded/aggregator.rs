//! Embedded dispatch for recurser-aggregator operations.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::ProofmanOptions;
use recurser::prove::{run_prove_recurser_aggregator, ProveRecurserAggregatorOptions};
use recurser::setup::{run_setup_recurser_aggregator, SetupRecurserAggregatorOptions};
use zisk_common::{Proof, StatsCostPerType};
use zisk_prover_backend::ProveOutput;

use super::EmbeddedClient;
use crate::aggregator::{recurser_setup_dir, RecurserAggregator};
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::prove::ProveResult;
use crate::setup::SetupResult;
use crate::upload::UploadResult;
use crate::JobEvent;

impl EmbeddedClient {
    pub(crate) fn do_upload_aggregator(&self, agg: &RecurserAggregator) -> Result<UploadResult> {
        Ok(UploadResult::new(agg.recurser_id.clone()))
    }

    pub(crate) fn do_setup_aggregator(
        &self,
        agg: &RecurserAggregator,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let agg = agg.clone();
        let subs_cloned = Arc::clone(&subs);

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);
            let result = run_setup_aggregator_blocking(&agg);
            fire_result_event(&subs_cloned, &result);
            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_aggregate_proof(
        &self,
        agg: &RecurserAggregator,
        proof_a: &Proof,
        proof_b: &Proof,
        private_inputs: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        let agg = agg.clone();
        let vfp_a = proof_a.get_vadcop_final_proof()?;
        let vfp_b = proof_b.get_vadcop_final_proof()?;
        let private_inputs = private_inputs.to_vec();
        let subs_cloned = Arc::clone(&subs);
        let proving_key = self.proving_key.clone();
        let gpu = self.gpu;

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);
            let result = run_aggregate_proof_blocking(
                &proving_key,
                &agg,
                vfp_a,
                vfp_b,
                &private_inputs,
                root_c_recurser_agg,
                gpu,
            );
            fire_result_event(&subs_cloned, &result);
            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }
}

fn run_setup_aggregator_blocking(agg: &RecurserAggregator) -> Result<SetupResult> {
    let stem = recurser_setup_dir(&agg.output_dir, &agg.recurser_id).join("recurser_aggregator");
    if has_verkey(&stem) {
        tracing::info!(
            "Recurser-aggregator '{}' already set up at {}; skipping",
            agg.recurser_id,
            stem.display()
        );
        return Ok(SetupResult { job_id: None });
    }

    let opts = SetupRecurserAggregatorOptions {
        setup_dir: agg.setup_dir.clone(),
        output_dir: agg.output_dir.clone(),
        program_vks: agg.program_vks.clone(),
        n_private_inputs: agg.n_private_inputs,
        prepare_publics_template: recurser::template_files::write_optional(
            agg.prepare_publics_template.as_deref(),
            "prepare_publics.circom",
        )?,
        check_publics_template: recurser::template_files::write_optional(
            agg.check_publics_template.as_deref(),
            "check_publics.circom",
        )?,
        aggregate_publics_template: recurser::template_files::write_required(
            &agg.aggregate_publics_template,
            "aggregate_publics.circom",
        )?,
    };

    // Scoped 64 MB-stack rayon pool — proofman setup overflows the default
    // ~2 MB worker stack on Circom / STARK preprocessing. Prove doesn't need
    // this; it runs in FFI.
    let pool = rayon::ThreadPoolBuilder::new()
        .stack_size(64 * 1024 * 1024)
        .build()
        .map_err(|e| anyhow!("failed to build 64 MB-stack rayon pool: {e}"))?;
    pool.install(|| run_setup_recurser_aggregator(&opts))
        .context("recurser-aggregator setup failed")?;
    Ok(SetupResult { job_id: None })
}

fn run_aggregate_proof_blocking(
    proving_key: &Path,
    agg: &RecurserAggregator,
    proof_a: proofman_verifier::VadcopFinalProof,
    proof_b: proofman_verifier::VadcopFinalProof,
    private_inputs: &[u64],
    root_c_override: Option<[u64; 4]>,
    gpu: bool,
) -> Result<ProveResult> {
    let mut pm_options = ProofmanOptions::new();
    pm_options.gpu = gpu;
    let proofman = ProofMan::<Goldilocks>::new(proving_key.to_path_buf(), pm_options)
        .map_err(|e| anyhow!("ProofMan::new failed: {e}"))?;

    let opts = ProveRecurserAggregatorOptions {
        output_dir: &agg.output_dir,
        recurser_id: &agg.recurser_id,
        proof_a: &proof_a,
        proof_b: &proof_b,
        private_inputs,
        root_c_recurser_agg: root_c_override,
    };

    // Prove runs in FFI; no 64 MB rayon pool needed.
    let vfp = run_prove_recurser_aggregator(&proofman, &opts)
        .context("recurser-aggregator proof generation failed")?;

    // Aggregator's own verkey → output Proof's zisk_vk.
    let zisk_vk = agg.vk()?.vk;
    let proof = Proof::new_from_vadcop_proof(&vfp.proof_with_publics(), vfp.compressed, zisk_vk)?;

    Ok(ProveResult::from(ProveOutput::from_remote(
        proof,
        0,
        Duration::from_secs(0),
        StatsCostPerType::default(),
    )))
}

fn has_verkey(stem: &Path) -> bool {
    let mut path = stem.as_os_str().to_owned();
    path.push(".verkey.bin");
    Path::new(&path).is_file()
}
