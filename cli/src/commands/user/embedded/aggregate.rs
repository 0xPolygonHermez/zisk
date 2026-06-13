use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, EmbeddedClientBuilder, Proof, ProofExt, VerboseMode};

use crate::commands::user::recurser_common::{parse_free_inputs, parse_root_c, resolve_recurser};
use crate::common::resolve_output_path;
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Fold two proofs into one recurser proof locally.
///
/// Identifies the recurser by the same aggregation toml used for `setup
/// --aggregation` (the recurser_id derivation is deterministic). If the setup
/// artifacts are missing they are generated first.
pub(crate) struct ZiskEmbeddedAggregate {
    /// Aggregation definition: `<programs>/aggregations/<name>.toml`.
    #[arg(short = 'a', long = "aggregation")]
    aggregation: PathBuf,

    /// Resolve guest ELFs from the release profile instead of debug.
    #[arg(long, default_value_t = false)]
    release: bool,

    /// First input proof (a `cargo-zisk prove` / `aggregate` output file).
    #[arg(long = "proof-a")]
    proof_a: PathBuf,

    /// Second input proof.
    #[arg(long = "proof-b")]
    proof_b: PathBuf,

    /// Proof A's `freeInputs` as comma-separated decimal `u64`s.
    #[arg(long = "free-inputs-a")]
    free_inputs_a: Option<String>,

    /// Proof B's `freeInputs` as comma-separated decimal `u64`s.
    #[arg(long = "free-inputs-b")]
    free_inputs_b: Option<String>,

    /// `rootCRecurserAgg` as 4 comma-separated decimal limbs. Omit to read
    /// the recurser's own verkey.
    #[arg(long = "root-c-recurser-agg")]
    root_c_recurser_agg: Option<String>,

    /// Save the generated proof to the specified file path
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    proving_key: Option<PathBuf>,

    /// Use GPU acceleration
    #[arg(short = 'g', long)]
    gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskEmbeddedAggregate {
    pub(crate) fn run(&self) -> Result<()> {
        print_banner();
        print_banner_command(format!("{} Aggregate", "EMBEDDED".bold()));
        print_banner_field("Aggregation", self.aggregation.display());
        print_banner_field("Proof A", self.proof_a.display());
        print_banner_field("Proof B", self.proof_b.display());
        println!();

        setup_logger(VerboseMode::from(self.verbose));

        let agg = resolve_recurser(&self.aggregation, self.release)?;
        info!("Recurser ID: {}", agg.recurser_id());

        let proof_a = Proof::load(&self.proof_a)
            .with_context(|| format!("Failed to load proof_a: {}", self.proof_a.display()))?;
        let proof_b = Proof::load(&self.proof_b)
            .with_context(|| format!("Failed to load proof_b: {}", self.proof_b.display()))?;
        let free_a = parse_free_inputs(self.free_inputs_a.as_deref())?;
        let free_b = parse_free_inputs(self.free_inputs_b.as_deref())?;
        let root_c = self.root_c_recurser_agg.as_deref().map(parse_root_c).transpose()?;

        let mut builder = EmbeddedClientBuilder::default().verbose(self.verbose);
        if self.gpu {
            builder = builder.gpu();
        }
        if let Some(pk) = &self.proving_key {
            builder = builder.proving_key(pk.clone());
        }
        let client = builder.build()?;

        // Registers the recurser with this process's prover; runs the full
        // setup first if the artifacts are missing.
        client.setup(&agg).run_sync()?;

        // The embedded aggregate path submits via `spawn_blocking`, so the
        // JobHandle needs a (multi-threaded) runtime to drive it.
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        let result = runtime.block_on(async {
            let mut request = client.aggregate_proofs(
                &agg,
                proof_a.with_free_inputs(free_a),
                proof_b.with_free_inputs(free_b),
            );
            if let Some(limbs) = root_c {
                request = request.root_c_recurser_agg(limbs);
            }
            request.run()?.await
        })?;

        let output_file = resolve_output_path(self.output.clone(), result.job_id());
        result.save_proof(&output_file).map_err(|e| {
            anyhow::anyhow!("Failed to save proof to {}: {e}", output_file.display())
        })?;

        info!("{}", "--- AGGREGATE SUMMARY ---------".bright_green().bold());
        info!("Recurser proof saved to {}", output_file.display());
        Ok(())
    }
}
