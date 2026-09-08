use anyhow::Result;
use pil2_stark_recurser::plonk2pil::setups::blake3::DEFAULT_LANES;
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

    /// Hash family (tree/transcript geometry), as in proofman-setup
    #[arg(long, default_value = proofman_common::hash_family::DEFAULT_HASH_ID, value_parser = clap::builder::PossibleValuesParser::new(proofman_common::hash_family::FAMILIES))]
    hash: String,

    /// Parallel BLAKE3 permutations per 56-row block the recursion is built at; only affects
    /// needsCompressor. blake3 family only; defaults to the recurser's own default.
    #[arg(long)]
    blake3_lanes: Option<usize>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskProofmanSetupStats {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        if let Some(l) = self.blake3_lanes {
            if self.hash != "blake3" {
                anyhow::bail!("--blake3-lanes only applies to --hash blake3, got {:?}", self.hash);
            }
            if !(1..=8).contains(&l) {
                anyhow::bail!(
                    "--blake3-lanes must be in 1..8 (the air's boundary depth caps it), got {l}"
                );
            }
        }

        let opts = StatsOptions {
            airout_path: self.airout.clone(),
            hash: self.hash.clone(),
            output_path: self.output.clone(),
            stark_structs_path: self.starkstructs.clone(),
            airgroups: self.airgroups.clone(),
            airs: self.airs.clone(),
            im_pols_stages: self.impols,
            blake3_lanes: self.blake3_lanes.unwrap_or(DEFAULT_LANES),
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
