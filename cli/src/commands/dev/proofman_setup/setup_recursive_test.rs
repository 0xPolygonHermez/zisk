use anyhow::Result;
use pil2_stark_setup::commands::setup_recursive_test::{
    run_setup_recursive_test, SetupRecursiveTestOptions,
};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Set up a test recursive circuit from a user-provided circom file.
pub(crate) struct ZiskProofmanSetupRecursiveTest {
    /// Build output directory
    #[arg(short = 'b', long)]
    build_dir: String,

    /// Path to the circom source file
    #[arg(short = 'c', long = "circom")]
    circom_path: String,

    /// Circuit name (e.g. "test")
    #[arg(short = 'n', long = "name")]
    circom_name: String,

    /// Setup type: compressor, aggregation, final_vadcop, or light
    #[arg(short = 't', long, default_value = "aggregation")]
    pub r#type: String,

    /// Hash function to use: Poseidon1, Poseidon2 or blake3
    #[arg(long, default_value = proofman_common::hash_family::DEFAULT_HASH_ID, value_parser = clap::builder::PossibleValuesParser::new(proofman_common::hash_family::FAMILIES))]
    pub hash: String,

    /// Parallel BLAKE3 permutations per 56-row block (1..8). blake3 family only; defaults to 4.
    #[arg(long)]
    blake3_lanes: Option<usize>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskProofmanSetupRecursiveTest {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        if let Some(l) = self.blake3_lanes {
            if !(1..=8).contains(&l) {
                anyhow::bail!(
                    "--blake3-lanes must be in 1..8 (the air's boundary depth caps it), got {l}"
                );
            }
        }

        let opts = SetupRecursiveTestOptions {
            hash: self.hash.clone(),
            build_dir: self.build_dir.clone(),
            circom_path: self.circom_path.clone(),
            circom_name: self.circom_name.clone(),
            setup_type: self.r#type.clone(),
            blake3_lanes: self.blake3_lanes,
        };
        run_setup_recursive_test(&opts)
    }
}
