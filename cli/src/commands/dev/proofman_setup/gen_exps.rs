use anyhow::Result;
use pil2_stark_setup::commands::gen_exps::{run_gen_exps, GenExpsOptions};
use std::path::PathBuf;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate per-AIR Q-expression CUDA kernels (.exps.so) for an existing
/// provingKey without re-running the full setup pipeline. No-op if nvcc is not
/// on PATH.
pub(crate) struct ZiskProofmanGenExps {
    /// Path to the `provingKey/` directory.
    #[arg(short = 'p', long = "proving-key")]
    proving_key: PathBuf,

    /// CUDA arch spec: auto | major | "89,120" | sm_120.
    #[arg(long, default_value = "auto")]
    arch: String,

    /// Skip an AIR whose Q has more than N ops (stays on the interpreter).
    #[arg(long, default_value_t = 60000)]
    cap: usize,

    /// Fixed ops/chunk for every AIR; omit to auto-tune the largest no-spill size.
    #[arg(long)]
    chunk: Option<usize>,

    /// pil2-stark source root for the nvcc includes (default: resolved automatically).
    #[arg(long)]
    stark_src: Option<PathBuf>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskProofmanGenExps {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = GenExpsOptions {
            proving_key: self.proving_key.clone(),
            arch: self.arch.clone(),
            cap: self.cap,
            chunk: self.chunk,
            stark_src: self.stark_src.clone(),
        };
        run_gen_exps(&opts)
    }
}
