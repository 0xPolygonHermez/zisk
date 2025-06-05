// extern crate env_logger;
use crate::commands::{cli_fail_if_macos, Field};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

use fields::Goldilocks;

use proofman::ProofMan;
use proofman_common::{ParamsGPU, VerboseMode};

use super::get_default_proving_key;

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
}

impl ZiskCheckSetup {
    pub fn run(&self) -> Result<()> {
        cli_fail_if_macos()?;

        println!("{} CheckSetup", format!("{: >12}", "Command").bright_green().bold());
        println!();

        let verbose_mode = VerboseMode::Debug;

        match self.field {
            Field::Goldilocks => ProofMan::<Goldilocks>::check_setup(
                self.get_proving_key(),
                self.aggregation,
                self.final_snark,
                ParamsGPU::default(),
                verbose_mode,
            )
            .map_err(|e| anyhow::anyhow!("Error checking setup: {}", e))?,
        };

        Ok(())
    }

    pub fn get_proving_key(&self) -> PathBuf {
        if self.proving_key.is_none() {
            get_default_proving_key()
        } else {
            self.proving_key.clone().unwrap()
        }
    }
}
