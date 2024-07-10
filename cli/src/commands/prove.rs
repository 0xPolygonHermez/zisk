// extern crate env_logger;
use clap::Parser;
use std::path::PathBuf;
use colored::Colorize;

use p3_goldilocks::Goldilocks;

use proofman::ProofMan;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ProveCmd {
    /// Proofman configuration file path
    #[clap(short, long)]
    pub lib: PathBuf,

    /// Public inputs file path
    #[clap(short, long)]
    pub public_inputs: Option<PathBuf>,

    /// Pilout path, here until setup ready
    #[clap(long)]
    pub pilout: PathBuf,

    /// Output file path
    #[clap(short, long, default_value = "proof.json")]
    pub output: PathBuf,
}

impl ProveCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} {}", format!("{: >12}", "Command").bright_green().bold(), "Prove");
        println!("");

        type GL = Goldilocks;

        let _proof = ProofMan::generate_proof::<GL>(
            self.lib.clone(),
            self.pilout.clone(),
            vec![1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0],
        );

        Ok(())
    }
}
