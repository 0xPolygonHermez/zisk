use anyhow::Result;
use pil2_stark_setup::commands::setup::{run_setup, SetupOptions};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

const DEFAULT_HASH: &str = "Poseidon1";

/// Parse a job-count flag, rejecting 0 (a 0-sized rayon/nvcc pool is invalid).
fn parse_jobs(s: &str) -> std::result::Result<usize, String> {
    let n: usize = s.parse().map_err(|_| format!("`{s}` is not a valid number"))?;
    if n == 0 {
        return Err("must be at least 1".to_string());
    }
    Ok(n)
}

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
    #[arg(long, default_value_t = 1, env = "RECURSIVE_JOBS", value_parser = parse_jobs)]
    recursive_jobs: usize,

    /// Max concurrent AIRs during non-recursive setup (default 1 = serial).
    /// Each slot runs pil_info + file I/O. Size by available RAM.
    #[arg(long, default_value_t = 1, env = "SETUP_JOBS", value_parser = parse_jobs)]
    setup_jobs: usize,

    /// Output file for per-AIR stats (same format as `stats` subcommand).
    /// If omitted, no stats file is written.
    #[arg(short = 'o', long)]
    output: Option<String>,

    /// Hash function to use: Poseidon1 or Poseidon2
    #[arg(long, default_value = DEFAULT_HASH, value_parser = ["Poseidon1", "Poseidon2"])]
    pub hash: String,

    /// Generate + compile per-AIR Q-expression CUDA kernels (.exps.so) at the end
    /// of setup. No-op if nvcc is not on PATH.
    #[arg(long, default_value_t = false)]
    gen_exps: bool,

    /// CUDA arch spec for --gen-exps: auto | major | "89,120" | sm_120.
    #[arg(long, default_value = "auto")]
    exps_arch: String,

    /// Skip an AIR whose Q has more than N ops (stays on the interpreter).
    #[arg(long, default_value_t = 40000)]
    exps_cap: usize,

    /// Fixed ops/chunk for every AIR; omit to auto-tune the largest no-spill size.
    #[arg(long)]
    exps_chunk: Option<usize>,

    /// pil2-stark source root for the nvcc includes (default: resolved automatically).
    #[arg(long)]
    exps_stark_src: Option<String>,

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
            gen_exps: self.gen_exps,
            exps_arch: self.exps_arch.clone(),
            exps_cap: self.exps_cap,
            exps_chunk: self.exps_chunk,
            exps_stark_src: self.exps_stark_src.clone(),
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
