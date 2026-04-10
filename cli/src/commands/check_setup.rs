// extern crate env_logger;
use crate::common::{get_proving_key, get_proving_key_snark};
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use zisk_build::ZISK_VERSION_MESSAGE;

use fields::Goldilocks;

use proofman::{check_setup_snark, ProofMan};
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Check that the proving key is correctly set up
pub struct ZiskCheckSetup {
    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Path to a precomputed PLONK proving key
    #[arg(short = 'w', long)]
    pub proving_key_plonk: Option<PathBuf>,

    /// Enable proofs aggregation
    #[arg(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    /// Enable PLONK proofs
    #[arg(short = 's', long, default_value_t = false)]
    pub plonk: bool,

    /// Use GPU acceleration
    #[clap(short = 'g', long, default_value_t = false)]
    pub gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ZiskCheckSetup {
    pub fn run(&self) -> Result<()> {
        println!("{} CheckSetup", format!("{: >12}", "Command").bright_green().bold());
        println!();

        setup_logger(self.verbose.into());

        ProofMan::<Goldilocks>::check_setup(
            get_proving_key(self.proving_key.as_ref()),
            self.aggregation,
            self.verbose.into(),
            self.gpu,
        )
        .map_err(|e| anyhow::anyhow!("Error checking setup: {}", e))?;

        if self.plonk {
            check_setup_snark::<Goldilocks>(
                &get_proving_key_snark(self.proving_key_plonk.as_ref()),
                self.verbose.into(),
                self.gpu,
            )
            .map_err(|e| anyhow::anyhow!("Error checking setup snark: {}", e))?
        }

        Ok(())
    }
}
