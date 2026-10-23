use anyhow::Result;
use pil2_stark_setup::commands::setup_compressed_final::{
    run_setup_compressed_final, SetupCompressedFinalOptions,
};
use pil2_stark_setup::types::stark_struct::LowDegreeTestKind;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Re-run only the `vadcop_final_compressed` stage on top of an existing
/// provingKey/<name>/vadcop_final/. Useful for iterating on compressed_final
/// without re-running the full recursive setup.
pub(crate) struct ZiskProofmanSetupCompressedFinal {
    /// Build directory containing `provingKey/<name>/vadcop_final/`.
    #[arg(short = 'b', long)]
    build_dir: String,

    /// Low-degree test of the compressed proof: FRI (default) or STIR
    #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(["FRI", "STIR"]))]
    low_degree_test: Option<String>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskProofmanSetupCompressedFinal {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let low_degree_test = self.low_degree_test.as_deref().map(|t| match t {
            "STIR" => LowDegreeTestKind::Stir,
            _ => LowDegreeTestKind::Fri,
        });
        let opts =
            SetupCompressedFinalOptions { build_dir: self.build_dir.clone(), low_degree_test };
        run_setup_compressed_final(&opts)
    }
}
