// extern crate env_logger;
use anyhow::Result;
use bytemuck::cast_slice;
use clap::Parser;
use colored::Colorize;
use fields::Goldilocks;
use std::io::Read;
use std::path::PathBuf;

use crate::ux::print_banner;
use proofman::SnarkWrapper;
use std::fs::File;

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

    #[clap(short = 'j', long, default_value_t = false)]
    pub save_json: bool,
}

impl ZiskProveSnark {
    pub fn run(&self) -> Result<()> {
        println!("{} ProveSnark", format!("{: >12}", "Command").bright_green().bold());
        println!();

        print_banner();

        let mut proof_file = File::open(&self.proof)?;
        let mut proof_u64 = Vec::new();
        proof_file.read_to_end(&mut proof_u64)?;
        let proof = cast_slice::<u8, u64>(&proof_u64);

        let snark_wrapper: SnarkWrapper<Goldilocks> =
            SnarkWrapper::new(&self.proving_key_snark, self.verbose.into())?;

        let snark_proof =
            snark_wrapper.generate_final_snark_proof(proof, &self.output_dir, self.save_json)?;
        println!(
            "{} Final SNARK proof generated. Proof: {:?}, Publics: {:?}",
            "Info:".bright_blue().bold(),
            snark_proof.proof_bytes,
            snark_proof.public_bytes
        );
        Ok(())
    }
}
