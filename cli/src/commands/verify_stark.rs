use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use colored::Colorize;
use proofman_common::initialize_logger;
use std::fs;
use verifier::verify;

use bytemuck::cast_slice;

use crate::ZISK_VERSION_MESSAGE;

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
pub struct ZiskVerify {
    #[clap(short = 'p', long)]
    pub proof: String,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ZiskVerify {
    pub fn run(&self) -> Result<()> {
        initialize_logger(self.verbose.into(), None);

        tracing::info!(
            "{}",
            format!("{} ZiskVerify", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        let start = std::time::Instant::now();

        let buffer = fs::read(&self.proof)?;
        let proof_slice: &[u64] = cast_slice(&buffer);

        let valid = verify(proof_slice);

        let elapsed = start.elapsed();

        if !valid {
            tracing::info!("{}", "\u{2717} Stark proof was not verified".bright_red().bold());
        } else {
            tracing::info!("{}", "\u{2713} Stark proof was verified".bright_green().bold());
        }

        tracing::info!(
            "{}",
            "--- VERIFICATION SUMMARY ---".bright_green().bold()
        );
        tracing::info!("      time: {} milliseconds", elapsed.as_millis());
        tracing::info!(
            "{}",
            "----------------------------".bright_green().bold()
        );

        if !valid {
            Err(anyhow!("Stark proof was not verified"))
        } else {
            Ok(())
        }
    }
}
