use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use colored::Colorize;
use proofman_common::initialize_logger;
use proofman_verifier::verify;
use std::fs;
use std::fs::File;
use std::io::Read;
use zstd::stream::read::Decoder;

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

    #[clap(short = 'k', long)]
    pub vk: String,

    #[clap(short = 'z', long, default_value_t = false)]
    pub zip: bool,
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

        let proof_buffer = if self.zip {
            // Read compressed proof and decompress it
            tracing::info!("Reading compressed proof file: {}", self.proof);
            let proof_file = File::open(self.proof.clone())?;
            let mut decoder = Decoder::new(proof_file)?;
            let mut proof_buffer = Vec::new();
            decoder.read_to_end(&mut proof_buffer)?;
            tracing::info!("Decompressed proof size: {} bytes", proof_buffer.len());
            proof_buffer
        } else {
            // Read uncompressed proof
            tracing::info!("Reading uncompressed proof file: {}", self.proof);
            let mut proof_file = File::open(self.proof.clone())?;
            let mut proof_buffer = Vec::new();
            proof_file.read_to_end(&mut proof_buffer)?;
            tracing::info!("Proof size: {} bytes", proof_buffer.len());
            proof_buffer
        };
        let proof_slice: &[u64] = cast_slice(&proof_buffer);

        let vk_buffer = fs::read(&self.vk)?;
        let verkey: &[u64] = cast_slice(&vk_buffer);

        let valid = verify(proof_slice, verkey);

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
}
