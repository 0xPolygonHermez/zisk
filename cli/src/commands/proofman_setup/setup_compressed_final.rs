use anyhow::Result;
use pil2_stark_setup::commands::setup_compressed_final::{run_setup_compressed_final, SetupCompressedFinalOptions};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Re-run only the `vadcop_final_compressed` stage on top of an existing
/// provingKey/<name>/vadcop_final/. Useful for iterating on compressed_final
/// without re-running the full recursive setup.
pub struct ZiskProofmanSetupCompressedFinal {
    /// Build directory containing `provingKey/<name>/vadcop_final/`.
    #[arg(short = 'b', long)]
    pub build_dir: String,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskProofmanSetupCompressedFinal {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = SetupCompressedFinalOptions { build_dir: self.build_dir.clone() };
        run_setup_compressed_final(&opts)
    }
}
