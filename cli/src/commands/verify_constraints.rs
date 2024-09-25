// extern crate env_logger;
use clap::Parser;
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

                     //// Debug mode (-d, -dd)
                     // #[arg(short, long, action = clap::ArgAction::Count, help = "Increase debug level")]
                     // pub debug: u8, // Using u8 to hold the number of `-d`
}

impl VerifyConstraintsCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // env_logger::builder().filter_level(VerboseMode::from_u8(self.verbose).into());

        println!("{} VerifyConstraints", format!("{: >12}", "Command").bright_green().bold());
        println!();

        let debug_mode = match self.verbose {
            0 => 1, // Default to Error
            1 => 2, // -v
            2 => 3, // -vv _ => log::LevelFilter::Trace,
            _ => 1,
        };

        match self.field {
            Field::Goldilocks => ProofMan::<GL>::generate_proof(
                self.witness_lib.clone(),
                self.rom.clone(),
                self.public_inputs.clone(),
                self.proving_key.clone(),
                PathBuf::new(),
                ProofOptions::new(debug_mode, false, false),
            )?,
        };

        Ok(())
    }
}
