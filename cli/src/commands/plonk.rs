use anyhow::Result;
use colored::Colorize;
use fields::Goldilocks;
use std::path::PathBuf;

use crate::ux::{print_banner, print_banner_command};
use proofman::SnarkWrapper;
use zisk_sdk::{ZISK_VERSION_MESSAGE, ZiskProofWithPublicValues};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a PLONK proof from a STARK (VADCOP) proof
pub struct ZiskPlonk {
    /// Path to the STARK (VADCOP) proof file
    #[arg(short = 'p', long)]
    pub proof: PathBuf,

    /// Path to a precomputed PLONK proving key
    #[arg(short = 'w', long)]
    pub proving_key_plonk: PathBuf,

    /// Output dir path
    #[arg(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,

    /// Use GPU acceleration
    #[clap(long, default_value_t = false)]
    pub gpu: bool,

    /// Verbose (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskPlonk {
    pub fn run(&mut self) -> Result<()> {
        print_banner();

        print_banner_command("Prove PLONK");

        let zisk_proof = ZiskProofWithPublicValues::load(&self.proof).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load ZiskProofWithPublicValues from file {}: {}",
                self.proof.display(),
                e
            )
        })?;

        let snark_wrapper: SnarkWrapper<Goldilocks> =
            SnarkWrapper::new(&self.proving_key_plonk, self.verbose.into())?;

        let proof = zisk_proof.get_vadcop_final_proof()?;

        let snark_proof =
            snark_wrapper.generate_final_snark_proof(&proof, Some(self.output_dir.clone()))?;
        snark_proof.save(self.output_dir.join("final_plonk_proof.bin")).map_err(|e| {
            anyhow::anyhow!(
                "Failed to save final PLONK proof to output dir {}: {}",
                self.output_dir.join("final_plonk_proof.bin").display(),
                e
            )
        })?;
        println!(
            "{} Final PLONK proof generated. Proof: {:?}, Publics: {:?}",
            "Info:".bright_blue().bold(),
            snark_proof.proof_bytes,
            snark_proof.public_bytes
        );
        Ok(())
    }
}
