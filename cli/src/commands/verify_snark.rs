// extern crate env_logger;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use proofman::{verify_snark_proof, SnarkProof};
use proofman_common::initialize_logger;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskVerifySnark {
    #[clap(short = 'p', long)]
    pub proof: String,

    #[clap(short = 'k', long)]
    pub verkey: PathBuf,

    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ZiskVerifySnark {
    pub fn run(&self) -> Result<()> {
        println!("{} ZiskVerifySnark", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into(), None);

        let proof = SnarkProof::load(&self.proof).map_err(|e| {
            anyhow::anyhow!("Failed to load SnarkProof from file {}: {}", self.proof, e)
        })?;

        verify_snark_proof(&proof, &self.verkey).map_err(|e| {
            anyhow::anyhow!("SNARK proof verification failed for proof {}: {}", self.proof, e)
        })
    }
}
