use anyhow::Result;
use pil2_stark_setup::commands::setup_snark::{run_setup_snark, SetupSnarkOptions};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate final SNARK setup (recursivef + fflonk/plonk final).
pub struct ZiskProofmanSetupSnark {
    /// Build directory (must already contain provingKey/ from a previous setup run)
    #[arg(short = 'b', long)]
    pub build_dir: String,

    /// Powers-of-tau (.ptau) file for snarkjs setup
    #[arg(long)]
    pub powers_of_tau: Option<String>,

    /// Final SNARK type: plonk (default) or fflonk
    #[arg(long, default_value = "plonk")]
    pub final_snark: String,

    /// Path to publics hash info JSON (optional)
    #[arg(long)]
    pub publics_info: Option<String>,

    /// Only generate the recursivef step; skip the final SNARK
    #[arg(long)]
    pub only_recursive_final: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskProofmanSetupSnark {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = SetupSnarkOptions {
            build_dir: self.build_dir.clone(),
            powers_of_tau: self.powers_of_tau.clone(),
            final_snark: self.final_snark.clone(),
            publics_info: self.publics_info.clone(),
            only_recursive_final: self.only_recursive_final,
        };
        run_setup_snark(&opts)
    }
}
