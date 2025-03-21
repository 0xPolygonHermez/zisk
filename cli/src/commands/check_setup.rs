// extern crate env_logger;
use crate::commands::Field;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use proofman_common::{initialize_logger, DebugInfo};
use std::path::PathBuf;

use p3_goldilocks::Goldilocks;

use proofman::ProofMan;
use proofman_common::{ProofOptions, VerboseMode};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskCheckSetup {
    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: PathBuf,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'f', long, default_value_t = false)]
    pub final_snark: bool,
}

impl ZiskCheckSetup {
    pub fn run(&self) -> Result<()> {
        println!("{} CheckSetup", format!("{: >12}", "Command").bright_green().bold());
        println!();

        let verbose_mode = VerboseMode::Debug;

        initialize_logger(verbose_mode);

        match self.field {
            Field::Goldilocks => ProofMan::<Goldilocks>::check_setup(
                self.proving_key.clone(),
                ProofOptions::new(
                    false,
                    verbose_mode,
                    self.aggregation,
                    self.final_snark,
                    false,
                    DebugInfo::default(),
                ),
            )
            .map_err(|e| anyhow::anyhow!("Error checking setup: {}", e))?,
        };

        Ok(())
    }
}
