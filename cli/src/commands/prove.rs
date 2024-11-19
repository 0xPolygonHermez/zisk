// extern crate env_logger;
use clap::Parser;
use proofman_common::{initialize_logger, parse_cached_buffers};
use std::path::PathBuf;
use std::collections::HashMap;
use colored::Colorize;
use crate::commands::field::Field;

use p3_goldilocks::Goldilocks;

use proofman::ProofMan;
use proofman_common::ProofOptions;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ProveCmd {
    /// Witness computation dynamic library path
    #[clap(short, long)]
    pub witness_lib: PathBuf,

    /// ROM file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short, long)]
    pub rom: Option<PathBuf>,

    /// Public inputs path
    #[clap(short = 'i', long)]
    pub public_inputs: Option<PathBuf>,

    /// Cached buffer path
    #[clap(short = 'c', long, value_parser = parse_cached_buffers)]
    pub cached_buffers: Option<HashMap<String, PathBuf>>,

    /// Setup folder path
    #[clap(long)]
    pub proving_key: PathBuf,

    /// Output dir path
    #[clap(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'd', long, default_value_t = false)]
    pub debug: bool,
    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ProveCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Prove", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into());

        if Path::new(&self.output_dir.join("proofs")).exists() {
            fs::remove_dir_all(self.output_dir.join("proofs")).expect("Failed to remove the proofs directory");
        }

        fs::create_dir_all(self.output_dir.join("proofs")).expect("Failed to create the proofs directory");

        match self.field {
            Field::Goldilocks => ProofMan::<Goldilocks>::generate_proof(
                self.witness_lib.clone(),
                self.rom.clone(),
                self.public_inputs.clone(),
                self.cached_buffers.clone(),
                self.proving_key.clone(),
                self.output_dir.clone(),
                ProofOptions::new(false, self.verbose.into(), self.aggregation, self.debug),
            )?,
        };

        Ok(())
    }
}
