use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

mod rebuild_witness_libs;
mod setup;
mod setup_recursive_test;
mod setup_snark;
mod stats;

pub use rebuild_witness_libs::ZiskProofmanRebuildWitnessLibs;
pub use setup::ZiskProofmanSetupSetup;
pub use setup_recursive_test::ZiskProofmanSetupRecursiveTest;
pub use setup_snark::ZiskProofmanSetupSnark;
pub use stats::ZiskProofmanSetupStats;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Proofman proving-key setup commands (mirrors the proofman-setup binary)
pub struct ZiskProofmanSetup {
    #[command(subcommand)]
    pub command: ZiskProofmanSetupCommand,
}

#[derive(clap::Subcommand)]
pub enum ZiskProofmanSetupCommand {
    /// Run non-recursive (and optionally recursive) setup for all AIRs.
    Setup(ZiskProofmanSetupSetup),
    /// Compute per-AIR statistics (constraints, intermediate polynomials, etc.).
    Stats(ZiskProofmanSetupStats),
    /// Generate final SNARK setup (recursivef + fflonk/plonk final).
    SetupSnark(ZiskProofmanSetupSnark),
    /// Set up a test recursive circuit from a user-provided circom file.
    SetupRecursiveTest(ZiskProofmanSetupRecursiveTest),
    /// Rebuild every witness library (.so/.dylib) in an existing provingKey
    /// without re-running the full setup pipeline.
    RebuildWitnessLibs(ZiskProofmanRebuildWitnessLibs),
}

impl ZiskProofmanSetup {
    pub fn run(&mut self) -> Result<()> {
        // 64 MB rayon stack — expression trees in large AIRs (e.g. ZisK) are deep.
        // Idempotent across calls; ignore the "already initialized" error.
        rayon::ThreadPoolBuilder::new().stack_size(64 * 1024 * 1024).build_global().ok();

        match &mut self.command {
            ZiskProofmanSetupCommand::Setup(cmd) => cmd.run(),
            ZiskProofmanSetupCommand::Stats(cmd) => cmd.run(),
            ZiskProofmanSetupCommand::SetupSnark(cmd) => cmd.run(),
            ZiskProofmanSetupCommand::SetupRecursiveTest(cmd) => cmd.run(),
            ZiskProofmanSetupCommand::RebuildWitnessLibs(cmd) => cmd.run(),
        }
    }
}
