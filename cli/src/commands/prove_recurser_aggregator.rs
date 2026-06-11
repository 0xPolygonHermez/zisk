use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::{ProofmanOptions, VerboseMode};
use proofman_verifier::VadcopFinalProof;
use recurser::prove::{
    prove_recurser_aggregator, register_recurser_setup, ProveRecurserAggregatorOptions,
};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::ZiskPaths;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Fold two `vadcop_final`-shape proofs into one recurser proof.
/// See recurser/docs/aggregator-flow.md.
pub struct ZiskProveRecurserAggregator {
    /// Directory the recurser setup wrote its artifacts to.
    #[arg(short = 'o', long = "output-dir", default_value = "build")]
    pub output_dir: String,

    /// Content-addressed `<recurser-id>` segment under
    /// `<output-dir>/provingKey/recurser/`. Logged by setup.
    #[arg(long = "recurser-id")]
    pub recurser_id: String,

    /// First input proof.
    #[arg(short = 'a', long = "proof-a")]
    pub proof_a: PathBuf,

    /// Second input proof.
    #[arg(short = 'b', long = "proof-b")]
    pub proof_b: PathBuf,

    /// Where to write the resulting `VadcopFinalProof`.
    #[arg(long = "output", default_value = "recurser_aggregator_proof.bin")]
    pub output: PathBuf,

    /// `rootCRecurserAgg` as 4 comma-separated decimal limbs. Omit to read
    /// the recurser's own `recurser_aggregator.verkey.bin`.
    #[arg(long = "root-c-recurser-agg")]
    pub root_c_recurser_agg: Option<String>,

    /// Proof A's `freeInputs` as comma-separated decimal `u64`s.
    #[arg(long = "free-inputs-a")]
    pub free_inputs_a: Option<String>,

    /// Proof B's `freeInputs` as comma-separated decimal `u64`s.
    #[arg(long = "free-inputs-b")]
    pub free_inputs_b: Option<String>,

    /// Use the GPU prover path.
    #[arg(long, default_value_t = false)]
    pub gpu: bool,

    /// Path to a precomputed proving key.
    #[arg(short = 'k', long = "proving-key")]
    pub proving_key: Option<PathBuf>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskProveRecurserAggregator {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let proof_a = VadcopFinalProof::load(&self.proof_a)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .with_context(|| format!("Failed to load proof_a: {}", self.proof_a.display()))?;
        let proof_b = VadcopFinalProof::load(&self.proof_b)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .with_context(|| format!("Failed to load proof_b: {}", self.proof_b.display()))?;

        let free_inputs_a = parse_free_inputs(self.free_inputs_a.as_deref())?;
        let free_inputs_b = parse_free_inputs(self.free_inputs_b.as_deref())?;
        let root_c_override = self.root_c_recurser_agg.as_deref().map(parse_root_c).transpose()?;

        let proving_key = ZiskPaths::get_proving_key(self.proving_key.as_ref());
        let verbose_mode: VerboseMode = self.verbose.into();
        let mut pm_options = ProofmanOptions::new();
        pm_options.verbose_mode = verbose_mode;
        pm_options.gpu = self.gpu;

        tracing::info!("Initializing ProofMan against {}", proving_key.display());
        let proofman = ProofMan::<Goldilocks>::new(proving_key, pm_options)
            .map_err(|e| anyhow::anyhow!("ProofMan::new failed: {e}"))?;

        let registered = register_recurser_setup(&proofman, &self.output_dir, &self.recurser_id)?;

        let opts = ProveRecurserAggregatorOptions {
            registered: &registered,
            proof_a: &proof_a,
            proof_b: &proof_b,
            free_inputs_a: &free_inputs_a,
            free_inputs_b: &free_inputs_b,
            root_c_recurser_agg: root_c_override,
        };
        let out = prove_recurser_aggregator(&proofman, &opts)?;

        if let Some(parent) = self.output.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to mkdir {}", parent.display()))?;
        }
        out.save(&self.output)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .with_context(|| format!("Failed to save output proof to {}", self.output.display()))?;
        tracing::info!("Recurser proof written to {}", self.output.display());
        Ok(())
    }
}

fn parse_free_inputs(s: Option<&str>) -> Result<Vec<u64>> {
    match s {
        Some(s) if !s.trim().is_empty() => s
            .split(',')
            .map(|t| {
                t.trim()
                    .parse::<u64>()
                    .with_context(|| format!("free input '{t}' is not a valid u64"))
            })
            .collect(),
        _ => Ok(Vec::new()),
    }
}

fn parse_root_c(s: &str) -> Result<[u64; 4]> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        bail!("--root-c-recurser-agg needs 4 comma-separated limbs, got {}", parts.len());
    }
    let mut limbs = [0u64; 4];
    for (i, p) in parts.iter().enumerate() {
        limbs[i] = p
            .trim()
            .parse::<u64>()
            .with_context(|| format!("rootC limb #{} ('{}') is not a valid u64", i, p))?;
    }
    Ok(limbs)
}
