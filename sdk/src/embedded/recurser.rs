//! Embedded dispatch for recurser operations.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use zisk_common::{Proof, StatsCostPerType};
use zisk_prover_backend::ProveOutput;
use zisk_recurser::setup::{
    read_vadcop_final_verkey, run_setup_recurser_aggregator, SetupRecurserAggregatorOptions,
};

use super::{EmbeddedClient, EmbeddedProver};
use crate::job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList};
use crate::prove::ProveResult;
use crate::recurser::Recurser;
use crate::setup::SetupResult;
use crate::upload::UploadResult;
use crate::{JobEvent, Result, SdkError};

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
        let proving_key = self.proving_key.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);
            let result = run_setup_aggregation_program_blocking(&prover, &proving_key, &agg);
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
        let result = run_setup_aggregation_program_blocking(&self.prover, &self.proving_key, agg);
        fire_result_event(&subs, &result);
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_aggregate_proofs(
        &self,
        agg: &Recurser,
        proof_a: &Proof,
        proof_b: &Proof,
        free_a: &[u64],
        free_b: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        let agg = agg.clone();
        let vfp_a = proof_a.get_vadcop_final_proof().map_err(SdkError::backend)?;
        let vfp_b = proof_b.get_vadcop_final_proof().map_err(SdkError::backend)?;
        let free_a = free_a.to_vec();
        let free_b = free_b.to_vec();
        let subs_cloned = Arc::clone(&subs);
        let prover = Arc::clone(&self.prover);

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);
            let result = run_aggregate_proofs_blocking(
                &prover,
                &agg,
                vfp_a,
                vfp_b,
                &free_a,
                &free_b,
                root_c_recurser_agg,
            );
            fire_result_event(&subs_cloned, &result);
            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }
}

/// Reject a recurser built against a different proving key than this client proves with.
///
/// `AggregationProgram` binds to the global key at build time; `EmbeddedClientBuilder::proving_key`
/// can put the prover on another. The `recurser_id` is content-addressed on the build-time key's
/// vadcop_final verkey, so the key cannot be swapped here -- only refused. Compares verkeys, not
/// paths (two can name one key) or hash families (two keys can share one).
fn ensure_recurser_matches_client_key(client_proving_key: &Path, agg: &Recurser) -> Result<()> {
    let client_key = client_proving_key
        .to_str()
        .ok_or_else(|| SdkError::Recurser("client proving key path is not valid UTF-8".into()))?;
    if client_key == agg.proving_key {
        return Ok(());
    }

    let read = |key: &str| {
        read_vadcop_final_verkey(key).map_err(|e| {
            SdkError::Recurser(format!("failed to read the vadcop_final verkey at {key} ({e})"))
        })
    };
    if read(client_key)? != read(&agg.proving_key)? {
        return Err(SdkError::Recurser(format!(
            "recurser '{}' was built against the proving key at {}, but this client proves with \
             the key at {}. Their vadcop_final verkeys differ, so proofs from this client cannot \
             verify under the recurser's. Build the aggregation program on the same key the \
             client uses, or drop the custom proving key.",
            agg.recurser_id, agg.proving_key, client_key,
        )));
    }
    Ok(())
}

fn run_setup_aggregation_program_blocking(
    prover: &EmbeddedProver,
    client_proving_key: &Path,
    agg: &Recurser,
) -> Result<SetupResult> {
    // Ahead of the is_active() early return: registering a mismatched cached recurser is
    // as wrong as generating one.
    ensure_recurser_matches_client_key(client_proving_key, agg)?;

    let artifacts =
        zisk_recurser::artifacts::RecurserArtifacts::new(&agg.output_dir, &agg.recurser_id);
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
            .map_err(|e| SdkError::Recurser(format!("registration failed: {e}")))?;
        return Ok(SetupResult { job_id: None });
    }

    let opts = SetupRecurserAggregatorOptions {
        proving_key: agg.proving_key.clone(),
        output_dir: agg.output_dir.clone(),
        templates: agg.templates.clone(),
    };

    // Scoped 64 MB-stack rayon pool — proofman setup overflows the default
    // ~2 MB worker stack on Circom / STARK preprocessing. Prove doesn't need
    // this; it runs in FFI.
    let pool = rayon::ThreadPoolBuilder::new()
        .stack_size(64 * 1024 * 1024)
        .build()
        .map_err(SdkError::backend)?;
    pool.install(|| run_setup_recurser_aggregator(&opts))
        .map_err(|e| SdkError::Recurser(format!("setup failed: {e:#}")))?;

    // Register the freshly-generated setup with proofman so it can prove.
    prover
        .register_recurser(&agg.output_dir, &agg.recurser_id)
        .map_err(|e| SdkError::Recurser(format!("registration failed: {e}")))?;
    Ok(SetupResult { job_id: None })
}

fn run_aggregate_proofs_blocking(
    prover: &EmbeddedProver,
    agg: &Recurser,
    proof_a: proofman_verifier::VadcopFinalProof,
    proof_b: proofman_verifier::VadcopFinalProof,
    free_a: &[u64],
    free_b: &[u64],
    root_c_override: Option<[u64; 4]>,
) -> Result<ProveResult> {
    let vfp = prover
        .prove_recurser(&agg.recurser_id, &proof_a, &proof_b, free_a, free_b, root_c_override)
        .map_err(|e| SdkError::Recurser(format!("proof generation failed: {e}")))?;

    // Recurser's own verkey → output Proof's zisk_vk.
    let zisk_vk = agg.vk()?.vk;
    // The proof's hash family travels on the VadcopFinalProof (stamped by
    // proofman from the recurser's proving key); carry it onto the Proof.
    let proof = Proof::new_from_vadcop_proof(
        &vfp.proof_with_publics(),
        vfp.compressed,
        zisk_vk,
        vfp.hash.clone(),
    )
    .map_err(SdkError::backend)?;

    Ok(ProveResult::from(ProveOutput::from_remote(
        proof,
        0,
        Duration::from_secs(0),
        StatsCostPerType::default(),
    )))
}
