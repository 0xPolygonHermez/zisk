use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use colored::Colorize;
use proofman_common::initialize_logger;
use verifier::verify;
use std::fs;

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

        let buffer = fs::read(&self.proof)?;
        let proof_slice: &[u64] = cast_slice(&buffer);

        let valid = verify(proof_slice);

        if !valid {
            tracing::info!(
                "VStark  : ··· {}",
                "\u{2717} Stark proof was not verified".bright_red().bold()
            );
            Err(anyhow!("Stark proof was not verified"))
        } else {
            tracing::info!(
                "VStark  :     {}",
                "\u{2713} Stark proof was verified".bright_green().bold()
            );
            Ok(())
        }
    }
}
