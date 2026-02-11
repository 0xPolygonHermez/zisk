use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{
    get_proving_key, setup_logger, verify_zisk_proof_with_proving_key, ZiskProofWithPublicValues,
};

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
pub struct ZiskVerify {
    #[clap(short = 'p', long)]
    pub proof: String,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,
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

        let proof = ZiskProofWithPublicValues::load(&self.proof).map_err(|e| {
            anyhow::anyhow!("Error loading VADCoP final proof from {}: {}", &self.proof, e)
        })?;

        let result = verify_zisk_proof_with_proving_key(
            proof.get_proof(),
            proof.get_publics(),
            proof.get_program_vk(),
            get_proving_key(self.proving_key.as_ref()),
        );

        let elapsed = start.elapsed();

        if result.is_err() {
            tracing::info!("{}", "\u{2717} Stark proof was not verified".bright_red().bold());
        } else {
            tracing::info!("{}", "\u{2713} Stark proof was verified".bright_green().bold());
        }

        tracing::info!("{}", "--- VERIFICATION SUMMARY ---".bright_green().bold());
        tracing::info!("      time: {} milliseconds", elapsed.as_millis());
        tracing::info!("{}", "----------------------------".bright_green().bold());

        result
    }
}
