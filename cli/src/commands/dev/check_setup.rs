// extern crate env_logger;
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::ZiskPaths;

use fields::Goldilocks;

use proofman::{check_setup_snark, ProofMan};
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Check that the proving key is correctly set up
pub(crate) struct CheckSetupCmd {
    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    proving_key: Option<PathBuf>,

    /// Path to a precomputed PLONK proving key
    #[arg(short = 'w', long)]
    proving_key_plonk: Option<PathBuf>,

    /// Disable proofs aggregation
    #[arg(short = 'a', long)]
    no_aggregation: bool,

    /// Enable PLONK proofs
    #[arg(short = 's', long)]
    plonk: bool,

    /// Use GPU acceleration
    #[cfg(not(feature = "cpu-only"))]
    #[arg(short = 'g', long)]
    gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl CheckSetupCmd {
    pub(crate) fn run(&self) -> Result<()> {
        println!("{} CheckSetup", format!("{: >12}", "Command").bright_green().bold());
        println!();

        setup_logger(self.verbose.into());

        #[cfg(not(feature = "cpu-only"))]
        let gpu = self.gpu;
        #[cfg(feature = "cpu-only")]
        let gpu = false;

        ProofMan::<Goldilocks>::check_setup(
            ZiskPaths::get_proving_key(self.proving_key.as_ref()),
            !self.no_aggregation,
            self.verbose.into(),
            gpu,
        )
        .map_err(|e| anyhow::anyhow!("Error checking setup: {}", e))?;

        if self.plonk {
            check_setup_snark::<Goldilocks>(
                &ZiskPaths::get_proving_key_snark(self.proving_key_plonk.as_ref()),
                self.verbose.into(),
                gpu,
            )
            .map_err(|e| anyhow::anyhow!("Error checking setup snark: {}", e))?
        }

        Ok(())
    }
}
