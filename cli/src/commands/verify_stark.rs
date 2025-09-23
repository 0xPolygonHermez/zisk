use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use colored::Colorize;
use proofman_common::initialize_logger;
use proofman_verifier::verify;
use std::fs;

use crate::ZISK_VERSION_MESSAGE;

use super::get_default_verkey;

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
    pub vk: Option<String>,
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

        let proof = fs::read(&self.proof)?;

        let vk = &self.get_verkey();

        let valid = verify(&proof, vk);

        let elapsed = start.elapsed();

        if !valid {
            tracing::info!("{}", "\u{2717} Stark proof was not verified".bright_red().bold());
        } else {
            tracing::info!("{}", "\u{2713} Stark proof was verified".bright_green().bold());
        }

        tracing::info!("{}", "--- VERIFICATION SUMMARY ---".bright_green().bold());
        tracing::info!("      time: {} milliseconds", elapsed.as_millis());
        tracing::info!("{}", "----------------------------".bright_green().bold());

        if !valid {
            Err(anyhow!("Stark proof was not verified"))
        } else {
            Ok(())
        }
    }

    /// Gets the verification key
    /// Uses the default one if not specified by user.
    pub fn get_verkey(&self) -> Vec<u8> {
        let vk_file =
            if self.vk.is_none() { get_default_verkey() } else { self.vk.clone().unwrap() };
        fs::read(&vk_file).unwrap()
    }
}
