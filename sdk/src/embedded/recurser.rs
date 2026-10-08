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
        let vfp_a = proof_a.get_vadcop_final_proof_to_aggregate().map_err(SdkError::backend)?;
        let vfp_b = proof_b.get_vadcop_final_proof_to_aggregate().map_err(SdkError::backend)?;
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

/// Reject a recurser built against a different proving key than the one in use.
///
/// Compares current verkeys against the build-time one, never paths: a key can sit at several
/// paths, and a path can hold a key regenerated since the build.
fn ensure_recurser_matches_client_key(client_proving_key: &Path, agg: &Recurser) -> Result<()> {
    let client_key = client_proving_key
        .to_str()
        .ok_or_else(|| SdkError::Recurser("client proving key path is not valid UTF-8".into()))?;

    // The client proves with `client_key`; setup builds the circuit from `agg.proving_key`.
    for key in [client_key, &agg.proving_key] {
        let current = read_vadcop_final_verkey(key).map_err(|e| {
            SdkError::Recurser(format!("failed to read the vadcop_final verkey at {key} ({e})"))
        })?;
        if current != agg.zisk_vk {
            return Err(SdkError::Recurser(format!(
                "recurser '{}' was built against the proving key at {}; the key at {} no longer \
                 has that vadcop_final verkey, so its proofs cannot verify under the recurser's. \
                 Rebuild the aggregation program against the key in use.",
                agg.recurser_id, agg.proving_key, key,
            )));
        }
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

    // Compression strips the flag that marks this a fold, taking the recursion-domain
    // check in `Proof::verify` with it.
    if vfp.compressed {
        return Err(SdkError::Recurser(
            "recurser produced a compressed proof; the recursion-domain check in \
             Proof::verify cannot classify it"
                .to_string(),
        ));
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, OnceLock};

    /// Temp proving key whose vadcop_final verkey is `vk`. Clears the dir first, so reusing a
    /// tag rewrites the key in place and tests need no teardown.
    fn key_dir(tag: &str, vk: [u64; 4]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("zisk-recurser-{tag}-{}", std::process::id()));
        let final_dir = dir.join("pilout").join("vadcop_final");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&final_dir).unwrap();
        std::fs::write(dir.join("pilout.globalInfo.json"), r#"{"name":"pilout","hash":"blake3"}"#)
            .unwrap();
        std::fs::write(
            final_dir.join("vadcop_final.verkey.json"),
            serde_json::to_string(&vk).unwrap(),
        )
        .unwrap();
        dir
    }

    /// A recurser built against `built`, pinned to the verkey it held then.
    fn agg_for(built: &Path, zisk_vk: [u64; 4]) -> Recurser {
        Recurser {
            recurser_id: "rid".into(),
            templates: zisk_recurser::CircomTemplates {
                normalize: None,
                aggregate_publics: "// body".into(),
                n_free: 0,
                n_publics_agg: 6,
                program_vks: vec![],
            },
            proving_key: built.to_str().unwrap().to_string(),
            zisk_vk: zisk_vk.map(|w| w.to_string()),
            hash_mode: zisk_common::HashMode::default(),
            output_dir: "/tmp/zisk-test-output".into(),
            vk_cache: Arc::new(OnceLock::new()),
        }
    }

    #[test]
    fn unchanged_key_is_accepted() {
        let dir = key_dir("unchanged", [1, 2, 3, 4]);
        ensure_recurser_matches_client_key(&dir, &agg_for(&dir, [1, 2, 3, 4])).unwrap();
    }

    /// What path comparison misses: same path, key regenerated since the build.
    #[test]
    fn key_regenerated_in_place_is_rejected() {
        let dir = key_dir("regenerated", [1, 2, 3, 4]);
        let agg = agg_for(&dir, [1, 2, 3, 4]);
        key_dir("regenerated", [9, 9, 9, 9]);
        ensure_recurser_matches_client_key(&dir, &agg).unwrap_err();
    }

    #[test]
    fn another_path_holding_the_same_key_is_accepted() {
        let built = key_dir("same-built", [1, 2, 3, 4]);
        let client = key_dir("same-client", [1, 2, 3, 4]);
        ensure_recurser_matches_client_key(&client, &agg_for(&built, [1, 2, 3, 4])).unwrap();
    }

    #[test]
    fn a_client_key_with_another_verkey_is_rejected() {
        let built = key_dir("other-built", [1, 2, 3, 4]);
        let client = key_dir("other-client", [5, 6, 7, 8]);
        ensure_recurser_matches_client_key(&client, &agg_for(&built, [1, 2, 3, 4])).unwrap_err();
    }

    /// Setup builds the circuit from `agg.proving_key`, so it is checked too.
    #[test]
    fn a_regenerated_setup_key_is_rejected_even_when_the_client_key_is_good() {
        let client = key_dir("setup-client", [1, 2, 3, 4]);
        let built = key_dir("setup-built", [1, 2, 3, 4]);
        let agg = agg_for(&built, [1, 2, 3, 4]);
        key_dir("setup-built", [9, 9, 9, 9]);
        let err = ensure_recurser_matches_client_key(&client, &agg).unwrap_err();
        assert!(err.to_string().contains(built.to_str().unwrap()), "{err}");
    }
}
