// extern crate env_logger;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use fields::Goldilocks;
use std::path::PathBuf;

use crate::ux::{print_banner, print_banner_command};
use proofman::SnarkWrapper;
use zisk_common::ZiskProofWithPublicValues;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskPlonk {
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

    #[clap(short = 'g', long, default_value_t = false)]
    pub gpu: bool,
}

impl ZiskPlonk {
    pub fn run(&self) -> Result<()> {
        print_banner();

        print_banner_command("Prove SNARK");

        let zisk_proof = ZiskProofWithPublicValues::load(&self.proof).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load ZiskProofWithPublicValues from file {}: {}",
                self.proof,
                e
            )
        })?;

        let snark_wrapper: SnarkWrapper<Goldilocks> =
            SnarkWrapper::new(&self.proving_key_snark, self.verbose.into(), true, self.gpu)?;

        let proof = zisk_proof.get_vadcop_final_proof()?;

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
