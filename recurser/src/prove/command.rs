use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use proofman::ProofMan;
use proofman_verifier::VadcopFinalProof;

use super::validate::{validate_prove_inputs, ProgramVkOrigin};
use crate::manifest::RecurserManifest;

const PROGRAM_VK_LEN: usize = 4;

pub struct ProveRecurserAggregatorOptions<'a> {
    /// Same `<output_dir>` the setup ran against.
    pub output_dir: &'a str,
    /// Content-addressed id logged at the end of setup.
    pub recurser_id: &'a str,
    pub proof_a: &'a VadcopFinalProof,
    pub proof_b: &'a VadcopFinalProof,
    pub private_inputs: &'a [u64],
    /// When `None`, defaults to the recurser's own `recurser_aggregator.verkey.bin`.
    pub root_c_recurser_agg: Option<[u64; PROGRAM_VK_LEN]>,
}

pub fn run_prove_recurser_aggregator<F: PrimeField64>(
    proofman: &ProofMan<F>,
    opts: &ProveRecurserAggregatorOptions<'_>,
) -> Result<VadcopFinalProof>
where
    GoldilocksQuinticExtension: ExtensionField<F>,
{
    let setup_dir = recurser_setup_dir(opts.output_dir, opts.recurser_id);
    let setup_stem = setup_dir.join("recurser_aggregator");

    if !sibling_exists(&setup_stem, ".verkey.json") {
        bail!(
            "Recurser setup not found at {:?}. Run `cargo-zisk setup-recurser-aggregator` first.",
            setup_stem
        );
    }

    let manifest = RecurserManifest::load(&setup_dir).with_context(|| {
        format!("Failed to load recurser manifest for id '{}'", opts.recurser_id)
    })?;

    let root_c = match opts.root_c_recurser_agg {
        Some(r) => r,
        None => read_verkey_4(&setup_stem)
            .context("Failed to default rootCRecurserAgg from recurser's verkey.bin")?,
    };

    let (origin_a, origin_b) = validate_prove_inputs(
        &manifest.inputs,
        &opts.proof_a.public_values,
        &opts.proof_b.public_values,
        opts.private_inputs,
        &root_c,
    )?;
    tracing::info!(
        "Proof classification: a={}, b={}",
        format_origin(origin_a),
        format_origin(origin_b),
    );

    proofman
        .register_recurser_setup(opts.recurser_id, &setup_stem)
        .map_err(|e| anyhow::anyhow!("register_recurser_setup failed: {e}"))?;

    tracing::info!("Proving recurser-aggregator '{}'", opts.recurser_id);
    let out = proofman
        .prove_recurser_aggregator(
            opts.recurser_id,
            opts.proof_a,
            opts.proof_b,
            opts.private_inputs,
            &root_c,
        )
        .map_err(|e| anyhow::anyhow!("prove_recurser_aggregator failed: {e}"))?;
    Ok(out)
}

fn recurser_setup_dir(output_dir: &str, recurser_id: &str) -> PathBuf {
    PathBuf::from(output_dir).join("provingKey").join("recurser").join(recurser_id)
}

fn sibling_exists(stem: &Path, ext: &str) -> bool {
    let mut p = stem.as_os_str().to_owned();
    p.push(ext);
    Path::new(&p).is_file()
}

fn read_verkey_4(stem: &Path) -> Result<[u64; PROGRAM_VK_LEN]> {
    let mut path = stem.as_os_str().to_owned();
    path.push(".verkey.bin");
    let mut file =
        fs::File::open(Path::new(&path)).with_context(|| format!("Failed to open {:?}", path))?;
    let mut bytes = [0u8; PROGRAM_VK_LEN * 8];
    file.read_exact(&mut bytes)
        .with_context(|| format!("Failed to read {} bytes from {:?}", bytes.len(), path))?;
    let mut limbs = [0u64; PROGRAM_VK_LEN];
    for i in 0..PROGRAM_VK_LEN {
        let chunk: [u8; 8] = bytes[i * 8..(i + 1) * 8].try_into().unwrap();
        limbs[i] = u64::from_le_bytes(chunk);
    }
    Ok(limbs)
}

fn format_origin(origin: ProgramVkOrigin) -> String {
    match origin {
        ProgramVkOrigin::RegisteredProgram(idx) => format!("leaf(program #{idx})"),
        ProgramVkOrigin::PriorAggregation => "aggregated".to_string(),
    }
}
