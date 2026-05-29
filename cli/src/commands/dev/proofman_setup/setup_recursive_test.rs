use anyhow::Result;
use pil2_stark_setup::commands::setup_recursive_test::{
    run_setup_recursive_test, SetupRecursiveTestOptions,
};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Set up a test recursive circuit from a user-provided circom file.
pub struct ZiskProofmanSetupRecursiveTest {
    /// Build output directory
    #[arg(short = 'b', long)]
    pub build_dir: String,

    /// Path to the circom source file
    #[arg(short = 'c', long = "circom")]
    pub circom_path: String,

    /// Circuit name (e.g. "test")
    #[arg(short = 'n', long = "name")]
    pub circom_name: String,

    /// Setup type: compressor, aggregation, final_vadcop, or light
    #[arg(short = 't', long, default_value = "aggregation")]
    pub r#type: String,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskProofmanSetupRecursiveTest {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = SetupRecursiveTestOptions {
            build_dir: self.build_dir.clone(),
            circom_path: self.circom_path.clone(),
            circom_name: self.circom_name.clone(),
            setup_type: self.r#type.clone(),
        };
        run_setup_recursive_test(&opts)
    }
}
