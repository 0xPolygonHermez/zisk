// extern crate env_logger;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use fields::Goldilocks;
use std::path::PathBuf;

use crate::ux::print_banner;
use proofman::SnarkWrapper;
use proofman_util::VadcopFinalProof;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskProveSnark {
    #[clap(short = 'p', long)]
    pub proof: String,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key_snark: PathBuf,

    /// Output dir path
    #[clap(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,

    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ZiskProveSnark {
    pub fn run(&self) -> Result<()> {
        println!("{} ProveSnark", format!("{: >12}", "Command").bright_green().bold());
        println!();

        print_banner();

        let proof = VadcopFinalProof::load(&self.proof).map_err(|e| {
            anyhow::anyhow!("Failed to load VadcopFinalProof from file {}: {}", self.proof, e)
        })?;

        let snark_wrapper: SnarkWrapper<Goldilocks> =
            SnarkWrapper::new(&self.proving_key_snark, self.verbose.into())?;

        let snark_proof =
            snark_wrapper.generate_final_snark_proof(&proof, Some(self.output_dir.clone()))?;
        snark_proof.save(self.output_dir.join("final_snark_proof.bin")).map_err(|e| {
            anyhow::anyhow!(
                "Failed to save final SNARK proof to output dir {}: {}",
                self.output_dir.join("final_snark_proof.bin").display(),
                e
            )
        })?;
        println!(
            "{} Final SNARK proof generated. Proof: {:?}, Publics: {:?}",
            "Info:".bright_blue().bold(),
            snark_proof.proof_bytes,
            snark_proof.public_bytes
        );
        Ok(())
    }
}
