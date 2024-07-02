// extern crate env_logger;
use clap::{Parser};
use pil2_stark::Pil2StarkProver;
use tinytemplate::error;
use std::{fmt::Error, path::PathBuf};
use colored::Colorize;
use goldilocks::Goldilocks;

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

        type Field = Goldilocks;

        let _proof =
            Pil2StarkProver::<Field>::prove(self.lib.clone(), self.pilout.clone(), self.public_inputs.clone())?;

        // TODO! Save proof

        Ok(())
    }
}
