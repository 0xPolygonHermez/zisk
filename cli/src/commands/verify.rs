use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::{ZiskProof, ZiskProofWithPublicValues};
use zisk_prover_backend::setup_logger;

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Verify a proof
pub struct ZiskVerify {
    #[clap(short = 'p', long)]
    pub proof: String,

    /// Verbose (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count)]
    pub verbose: u8, // Using u8 to hold the number of `-v`
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

        let proof = ZiskProofWithPublicValues::load(&self.proof)
            .map_err(|e| anyhow::anyhow!("Error loading proof from {}: {}", &self.proof, e))?;

        let proof_type = match &proof.get_proof() {
            ZiskProof::VadcopFinal(_) | ZiskProof::VadcopFinalMinimal(_) => "STARK",
            ZiskProof::Plonk(_) => "PLONK",
            _ => panic!("Unsupported proof type"),
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
