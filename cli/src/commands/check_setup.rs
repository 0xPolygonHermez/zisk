// extern crate env_logger;
use crate::commands::{get_proving_key, get_proving_key_snark};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

use fields::Goldilocks;

use proofman::{check_setup_snark, ProofMan};
use proofman_common::initialize_logger;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskCheckSetup {
    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Setup folder path
    #[clap(short = 'w', long)]
    pub proving_key_snark: Option<PathBuf>,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 's', long, default_value_t = false)]
    pub snark: bool,

    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ZiskCheckSetup {
    pub fn run(&self) -> Result<()> {
        println!("{} CheckSetup", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into(), None);

        ProofMan::<Goldilocks>::check_setup(
            get_proving_key(self.proving_key.as_ref()),
            self.aggregation,
            self.verbose.into(),
        )
        .map_err(|e| anyhow::anyhow!("Error checking setup: {}", e))?;

        if self.snark {
            check_setup_snark::<Goldilocks>(
                &get_proving_key_snark(self.proving_key_snark.as_ref()),
                self.verbose.into(),
            )
            .map_err(|e| anyhow::anyhow!("Error checking setup snark: {}", e))?
        }

        Ok(())
    }
}
