use anyhow::Result;
use pil2_stark_setup::commands::stats::{run_stats, StatsOptions};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Compute per-AIR statistics (constraints, intermediate polynomials, etc.).
pub(crate) struct ZiskProofmanSetupStats {
    /// Path to compiled .pilout file
    #[arg(short = 'a', long)]
    airout: String,

    /// Output file for detailed stats (default: tmp/stats.txt)
    #[arg(short = 'o', long)]
    output: Option<String>,

    /// Path to starkstructs.json settings
    #[arg(short = 's', long)]
    starkstructs: Option<String>,

    /// Filter by airgroup names (repeat for multiple)
    #[arg(short = 'g', long = "airgroups", num_args = 1..)]
    airgroups: Vec<String>,

    /// Filter by air names (repeat for multiple)
    #[arg(short = 'i', long = "airs", num_args = 1..)]
    airs: Vec<String>,

    /// Show intermediate polynomial details per stage
    #[arg(short = 'm', long)]
    impols: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskProofmanSetupStats {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = StatsOptions {
            airout_path: self.airout.clone(),
            output_path: self.output.clone(),
            stark_structs_path: self.starkstructs.clone(),
            airgroups: self.airgroups.clone(),
            airs: self.airs.clone(),
            im_pols_stages: self.impols,
        };

        // Expression trees in large AIRs (e.g. ZisK) can be thousands of levels deep,
        // which overflows the default 8 MB main-thread stack. Run on a thread with the
        // same 64 MB stack used by the rayon pool.
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || run_stats(&opts))
            .expect("failed to spawn stats thread")
            .join()
            .expect("stats thread panicked")
    }
}
