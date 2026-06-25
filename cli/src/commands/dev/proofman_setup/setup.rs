use anyhow::Result;
use pil2_stark_setup::commands::setup::{run_setup, SetupOptions};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

const DEFAULT_HASH: &str = "Poseidon1";

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Run non-recursive (and optionally recursive) setup for all AIRs.
pub(crate) struct ZiskProofmanSetupSetup {
    /// Path to compiled .pilout file
    #[arg(short = 'a', long)]
    airout: String,

    /// Build output directory
    #[arg(short = 'b', long)]
    build_dir: String,

    /// Directory containing fixed column files
    #[arg(short = 'u', long)]
    fixed_dir: Option<String>,

    /// Enable recursive/aggregation setup
    #[arg(short = 'r', long)]
    recursive: bool,

    /// Path to starkstructs.json settings
    #[arg(short = 's', long)]
    stark_structs: Option<String>,

    /// Max concurrent recursive1 air pipelines (default 1 = serial).
    /// Each slot runs one circom compile + pil2com. Size by available RAM:
    /// set to floor(available_GB / per_air_peak_GB).
    #[arg(long, default_value_t = 1, env = "RECURSIVE_JOBS")]
    recursive_jobs: usize,

    /// Max concurrent AIRs during non-recursive setup (default 1 = serial).
    /// Each slot runs pil_info + file I/O. Size by available RAM.
    #[arg(long, default_value_t = 1, env = "SETUP_JOBS")]
    setup_jobs: usize,

    /// Output file for per-AIR stats (same format as `stats` subcommand).
    /// If omitted, no stats file is written.
    #[arg(short = 'o', long)]
    output: Option<String>,

    /// Hash function to use: Poseidon1 or Poseidon2
    #[arg(long, default_value = DEFAULT_HASH, value_parser = ["Poseidon1", "Poseidon2"])]
    pub hash: String,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskProofmanSetupSetup {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        let opts = SetupOptions {
            hash: self.hash.clone(),
            airout_path: self.airout.clone(),
            build_dir: self.build_dir.clone(),
            fixed_dir: self.fixed_dir.clone(),
            stark_structs_path: self.stark_structs.clone(),
            recursive: self.recursive,
            recursive_jobs: self.recursive_jobs,
            setup_jobs: self.setup_jobs,
            stats_output_path: self.output.clone(),
        };

        let result = run_setup(&opts);

        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmHWM:") || line.starts_with("VmPeak:") {
                    tracing::info!("{}", line.trim());
                }
            }
        }

        result
    }
}
