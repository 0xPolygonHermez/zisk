// extern crate env_logger;
use clap::Parser;
use proofman_common::{initialize_logger, json_to_debug_instances_map, DebugInfo};
use std::path::PathBuf;
use colored::Colorize;
use crate::commands::field::Field;

use p3_goldilocks::Goldilocks;

use proofman::ProofMan;
use proofman_common::ProofOptions;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct VerifyConstraintsCmd {
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

    /// Setup folder path
    #[clap(long)]
    pub proving_key: PathBuf,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,
}

impl VerifyConstraintsCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} VerifyConstraints", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into());

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => json_to_debug_instances_map(self.proving_key.clone(), debug_value.clone()),
        };

        match self.field {
            Field::Goldilocks => ProofMan::<Goldilocks>::generate_proof(
                self.witness_lib.clone(),
                self.rom.clone(),
                self.public_inputs.clone(),
                self.proving_key.clone(),
                PathBuf::new(),
                ProofOptions::new(true, self.verbose.into(), false, false, debug_info),
            )?,
        };

        Ok(())
    }
}
