// extern crate env_logger;
use crate::commands::{get_proving_key, Field};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

use fields::Goldilocks;

use proofman::ProofMan;
use proofman_common::initialize_logger;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskCheckSetup {
    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'f', long, default_value_t = false)]
    pub final_snark: bool,

    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ZiskCheckSetup {
    pub fn run(&self) -> Result<()> {
        println!("{} CheckSetup", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into(), None);

        match self.field {
            Field::Goldilocks => ProofMan::<Goldilocks>::check_setup(
                get_proving_key(self.proving_key.as_ref()),
                self.aggregation,
                self.final_snark,
                self.verbose.into(),
            )
            .map_err(|e| anyhow::anyhow!("Error checking setup: {}", e))?,
        };

        Ok(())
    }
}
