use anyhow::Result;
use pil2_stark_setup::commands::setup_snark::{run_setup_snark, SetupSnarkOptions};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate final SNARK setup (recursivef + fflonk/plonk final).
pub(crate) struct ZiskProofmanSetupSnark {
    /// Build directory (must already contain provingKey/ from a previous setup run)
    #[arg(short = 'b', long)]
    build_dir: String,

    /// Powers-of-tau (.ptau) file for snarkjs setup
    #[arg(long)]
    powers_of_tau: Option<String>,

    /// Final SNARK type: plonk (default) or fflonk
    #[arg(long, default_value = "plonk")]
    final_snark: String,

    /// Path to publics hash info JSON (optional)
    #[arg(long)]
    publics_info: Option<String>,

    /// Only generate the recursivef step; skip the final SNARK
    #[arg(long)]
    only_recursive_final: bool,

    /// pilfflonk: the extra scalar multiplications of its grouping, instead of the wrap family's
    #[arg(long)]
    extra_muls: Option<u64>,

    /// CUDA arch spec of the Q kernels (.exps.so), pilfflonk's wrap's and a blake3 key's recursivef's
    /// when setup-snark sets it up: auto | major | "89,120" | sm_120. No kernel without nvcc.
    #[arg(long, default_value = "auto")]
    exps_arch: String,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskProofmanSetupSnark {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = SetupSnarkOptions {
            build_dir: self.build_dir.clone(),
            powers_of_tau: self.powers_of_tau.clone(),
            final_snark: self.final_snark.clone(),
            publics_info: self.publics_info.clone(),
            only_recursive_final: self.only_recursive_final,
            extra_muls: self.extra_muls,
            exps_arch: self.exps_arch.clone(),
            contract_version: Some(format!("v{}", env!("CARGO_PKG_VERSION"))),
        };
        run_setup_snark(&opts)
    }
}
