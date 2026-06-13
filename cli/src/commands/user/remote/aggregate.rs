use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, Proof, ProofExt, RemoteClient, VerboseMode};

use crate::commands::user::recurser_common::{parse_free_inputs, parse_root_c, resolve_recurser};
use crate::common::resolve_output_path;
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Fold two proofs into one recurser proof on the remote service.
///
/// The recurser must already be set up (run `remote setup --aggregation` first).
pub(crate) struct ZiskRemoteAggregate {
    /// Aggregation definition: `<programs>/aggregations/<name>.toml`.
    #[arg(short = 'a', long = "aggregation")]
    aggregation: PathBuf,

    /// Resolve guest ELFs from the release profile instead of debug.
    #[arg(long, default_value_t = false)]
    release: bool,

    /// First input proof (a prove / aggregate output file).
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

    /// Proof timeout in seconds (0 = no timeout)
    #[arg(long, default_value_t = 0)]
    timeout: u64,
}

impl ZiskRemoteAggregate {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        print_banner();
        print_banner_command(format!("{} Aggregate", "REMOTE".bold()));
        print_banner_field("Aggregation", self.aggregation.display());
        print_banner_field("Proof A", self.proof_a.display());
        print_banner_field("Proof B", self.proof_b.display());
        println!();

        setup_logger(VerboseMode::Info);

        let agg = resolve_recurser(&self.aggregation, self.release)?;
        info!("Recurser ID: {}", agg.recurser_id());

        let proof_a = Proof::load(&self.proof_a)
            .with_context(|| format!("Failed to load proof_a: {}", self.proof_a.display()))?;
        let proof_b = Proof::load(&self.proof_b)
            .with_context(|| format!("Failed to load proof_b: {}", self.proof_b.display()))?;
        let free_a = parse_free_inputs(self.free_inputs_a.as_deref())?;
        let free_b = parse_free_inputs(self.free_inputs_b.as_deref())?;
        let root_c = self.root_c_recurser_agg.as_deref().map(parse_root_c).transpose()?;

        // Idempotent: makes sure the spec is registered with the coordinator.
        client.upload(&agg).run()?;

        let mut request = client.aggregate_proofs(
            &agg,
            proof_a.with_free_inputs(free_a),
            proof_b.with_free_inputs(free_b),
        );
        if let Some(limbs) = root_c {
            request = request.root_c_recurser_agg(limbs);
        }
        if self.timeout > 0 {
            request = request.timeout(Duration::from_secs(self.timeout));
        }
        let result = request.run()?.await?;

        let output_file = resolve_output_path(self.output.clone(), result.job_id());
        result.save_proof(&output_file).map_err(|e| {
            anyhow::anyhow!("Failed to save proof to {}: {e}", output_file.display())
        })?;

        info!("{}", "--- AGGREGATE SUMMARY ---------".bright_green().bold());
        info!("Recurser proof saved to {}", output_file.display());
        Ok(())
    }
}
