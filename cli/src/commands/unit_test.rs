use anyhow::Result;
use std::path::PathBuf;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::{BackendProverOpts, UnitTestProver};

use crate::ux::{print_banner, print_banner_command, print_banner_field, print_execution_summary};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Verify constraints for one or more state machines from a JSON file of typed inputs,
/// without an ELF or ROM execution.
///
/// The JSON shape is `{ "<TypeName>": [ <input>, ... ], ... }` where `<TypeName>` is one of
/// Binary, BinaryAdd, BinaryExtension, Arith, Keccakf, Sha256f, Poseidon2, Blake2,
/// ArithEq, ArithEq384, Add256, MemAlign.
pub struct ZiskUnitTest {
    /// JSON file containing typed state-machine inputs
    #[arg(short = 'i', long)]
    pub inputs: PathBuf,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Use GPU acceleration (requires CUDA-enabled build; implies packed traces)
    #[cfg(not(feature = "cpu-only"))]
    #[arg(short = 'g', long)]
    pub gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Path to a debug configuration file
    #[clap(short = 'd', long, hide = true)]
    pub debug: Option<Option<String>>,
}

impl ZiskUnitTest {
    pub fn run(&mut self) -> Result<()> {
        if !self.inputs.exists() {
            anyhow::bail!("Inputs file not found at {}", self.inputs.display());
        }

        print_banner();
        print_banner_command("Unit Test");
        print_banner_field("Inputs", self.inputs.display());

        let mut prover_options = BackendProverOpts::default().verbose(self.verbose);
        #[cfg(not(feature = "cpu-only"))]
        if self.gpu {
            prover_options = prover_options.gpu();
        }
        if let Some(ref path) = self.proving_key {
            prover_options = prover_options.proving_key(path.clone());
        }

        let prover = UnitTestProver::new(&prover_options)?;
        let result = prover.verify_constraints(self.inputs.clone(), self.debug.clone())?;

        info!("{}", "--- UNIT TEST SUMMARY -------------------------------");
        print_execution_summary(
            result.get_executor_time(),
            result.get_duration(),
            result.get_execution_steps(),
        );

        Ok(())
    }
}
