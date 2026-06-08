use anyhow::Result;
use pil2_stark_setup::commands::rebuild_witness::{run_rebuild_witness, RebuildWitnessOptions};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Rebuild every witness library (.so/.dylib) in an existing provingKey
/// without re-running the full setup pipeline.
pub struct ZiskProofmanRebuildWitnessLibs {
    /// Path to the `provingKey/` directory.
    #[arg(short = 'p', long = "proving-key")]
    pub proving_key: String,

    /// Number of circom compiles to run in parallel (default 1 = serial).
    /// Each circom invocation is single-threaded but RAM-hungry; size by
    /// available memory rather than CPU count.
    #[arg(short = 'j', long = "jobs", default_value_t = 1, env = "REBUILD_JOBS")]
    pub jobs: usize,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskProofmanRebuildWitnessLibs {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = RebuildWitnessOptions {
            proving_key: self.proving_key.clone(),
            build_dir: None,
            jobs: self.jobs,
        };
        run_rebuild_witness(&opts)
    }
}
