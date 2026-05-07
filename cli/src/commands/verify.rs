use anyhow::Result;
use colored::Colorize;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::{Proof, ProofKind};
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Verify a proof
pub struct ZiskVerify {
    /// Path to the proof file
    #[clap(short = 'p', long)]
    pub proof: String,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskVerify {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        tracing::info!(
            "{}",
            format!("{} ZiskVerify", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        let start = std::time::Instant::now();

        let proof = Proof::load(&self.proof)
            .map_err(|e| anyhow::anyhow!("Error loading proof from {}: {}", &self.proof, e))?;

        let proof_type = match proof.proof_kind {
            ProofKind::VadcopFinal | ProofKind::VadcopFinalMinimal => "STARK",
            ProofKind::Plonk => "PLONK",
        };

        let result = proof.verify();

        let elapsed = start.elapsed();

        if result.is_err() {
            tracing::info!(
                "{}",
                format!("\u{2717} {} proof was not verified", proof_type).bright_red().bold()
            );
        } else {
            tracing::info!(
                "{}",
                format!("\u{2713} {} proof was verified", proof_type).bright_green().bold()
            );
        }

        tracing::info!("{}", "--- VERIFICATION SUMMARY ---".bright_green().bold());
        tracing::info!("      time: {} milliseconds", elapsed.as_millis());
        tracing::info!("{}", "----------------------------".bright_green().bold());

        result
    }
}
