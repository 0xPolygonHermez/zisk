use anyhow::{bail, Context, Result};
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use proofman::ProofMan;
use proofman_verifier::VadcopFinalProof;

use super::validate::{validate_prove_inputs, ProgramVkOrigin};
use crate::artifacts::RecurserArtifacts;
use crate::manifest::RecurserManifest;

const PROGRAM_VK_LEN: usize = 4;

#[derive(Clone)]
pub struct RegisteredRecurser {
    recurser_id: String,
    manifest: RecurserManifest,
    verkey: [u64; PROGRAM_VK_LEN],
}

impl RegisteredRecurser {
    pub fn recurser_id(&self) -> &str {
        &self.recurser_id
    }

    pub fn manifest(&self) -> &RecurserManifest {
        &self.manifest
    }

    /// This recurser's verification key. It is also the default
    /// `rootCRecurserAgg` used when the caller does not pass one (an aggregated
    /// input proof carries this VK as its own `programVK`).
    pub fn verkey(&self) -> [u64; PROGRAM_VK_LEN] {
        self.verkey
    }
}

/// Register a previously-generated recurser setup with `proofman`.
///
/// This is the *only* prove-side function that knows where setup put its files.
/// It resolves the artifact layout, refuses to proceed unless setup actually
/// completed (see [`RecurserArtifacts::is_active`]), loads the manifest, and hands
/// the setup to proofman. proofman builds the const-tree on first registration
/// and persists it next to the other artifacts; later registrations load it
/// from disk.
///
/// Run this once per `recurser_id` per `ProofMan` instance; [`prove_recurser_aggregator`]
/// can then be called any number of times.
pub fn register_recurser_setup<F: PrimeField64>(
    proofman: &ProofMan<F>,
    output_dir: &str,
    recurser_id: &str,
) -> Result<RegisteredRecurser>
where
    GoldilocksQuinticExtension: ExtensionField<F>,
{
    let artifacts = RecurserArtifacts::new(output_dir, recurser_id);

    if !artifacts.is_active() {
        bail!(
            "Recurser setup '{}' is not ready at {:?}: setup has not completed for this id \
             (missing: {}). Run `cargo-zisk setup --aggregation` first.",
            recurser_id,
            artifacts.dir(),
            artifacts.missing_artifacts().join(", "),
        );
    }

    let manifest = RecurserManifest::load(artifacts.dir())
        .with_context(|| format!("Failed to load recurser manifest for id '{recurser_id}'"))?;

    let verkey = artifacts.read_verkey().context("Failed to read the recurser's verkey.bin")?;

    proofman
        .register_recurser_setup(recurser_id, &artifacts.setup_stem())
        .map_err(|e| anyhow::anyhow!("register_recurser_setup failed: {e}"))?;

    Ok(RegisteredRecurser { recurser_id: recurser_id.to_string(), manifest, verkey })
}

/// Inputs to a single recurser proof against an already-registered
/// setup. No filesystem or layout knowledge lives here — everything is the
/// validated, in-memory result of [`register_recurser_setup`] plus the proofs.
pub struct ProveRecurserAggregatorOptions<'a> {
    pub registered: &'a RegisteredRecurser,
    pub proof_a: &'a VadcopFinalProof,
    pub proof_b: &'a VadcopFinalProof,
    pub free_inputs_a: &'a [u64],
    pub free_inputs_b: &'a [u64],
    /// When `None`, defaults to the recurser's verkey (`registered.verkey()`).
    pub root_c_recurser_agg: Option<[u64; PROGRAM_VK_LEN]>,
}

/// Fold two `vadcop_final`-shape proofs into one against an already-registered
/// recurser setup.
///
/// Pure with respect to the on-disk layout: it validates the proofs against the
/// registered manifest and delegates to proofman. It does not read setup files
/// or register anything — call [`register_recurser_setup`] first.
pub fn prove_recurser_aggregator<F: PrimeField64>(
    proofman: &ProofMan<F>,
    opts: &ProveRecurserAggregatorOptions<'_>,
) -> Result<VadcopFinalProof>
where
    GoldilocksQuinticExtension: ExtensionField<F>,
{
    let registered = opts.registered;

    let root_c = opts.root_c_recurser_agg.unwrap_or_else(|| registered.verkey());

    if opts.proof_a.hash != opts.proof_b.hash {
        bail!(
            "Hash family mismatch between input proofs: a={:?}, b={:?}. \
             Both proofs must be produced with the same hash family.",
            opts.proof_a.hash,
            opts.proof_b.hash,
        );
    }

    let (origin_a, origin_b) = validate_prove_inputs(
        &registered.manifest().inputs,
        &opts.proof_a.public_values,
        &opts.proof_b.public_values,
        opts.free_inputs_a,
        opts.free_inputs_b,
        &root_c,
    )?;
    tracing::info!(
        "Proof classification: a={}, b={}",
        format_origin(origin_a),
        format_origin(origin_b),
    );

    // The circuit's per-side arrays are fixed at n_free_inputs; zero-pad each
    // side so callers (CLI, SDK) only supply what their proof's group consumes.
    let n_free_inputs = registered.manifest().inputs.n_free_inputs();
    let pad = |v: &[u64]| -> Vec<u64> {
        let mut padded = v.to_vec();
        padded.resize(n_free_inputs, 0);
        padded
    };
    let free_inputs_a = pad(opts.free_inputs_a);
    let free_inputs_b = pad(opts.free_inputs_b);

    tracing::info!("Proving recurser '{}'", registered.recurser_id());
    let out = proofman
        .prove_recurser_aggregator(
            registered.recurser_id(),
            opts.proof_a,
            opts.proof_b,
            &free_inputs_a,
            &free_inputs_b,
            &root_c,
        )
        .map_err(|e| anyhow::anyhow!("prove_recurser_aggregator failed: {e}"))?;
    Ok(out)
}

fn format_origin(origin: ProgramVkOrigin) -> String {
    match origin {
        ProgramVkOrigin::RegisteredProgram(idx) => format!("leaf(program #{idx})"),
        ProgramVkOrigin::PriorAggregation => "aggregated".to_string(),
    }
}
