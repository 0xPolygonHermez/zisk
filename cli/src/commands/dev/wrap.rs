use std::path::PathBuf;

use anyhow::Result;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::Proof;
use zisk_prover_backend::{BackendProverOpts, ProverClientBuilder};

use crate::common::resolve_output_path;
use crate::proof::{ensure_snark_key, select_wrap_kind, wrap_kind_label, SnarkArgs};
use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a SNARK/minimal proof from a STARK (VADCOP) proof
pub(crate) struct WrapCmd {
    /// Path to the STARK (VADCOP) proof file
    #[arg(short = 'p', long)]
    proof: PathBuf,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    proving_key: Option<PathBuf>,

    /// Path to a precomputed SNARK proving key
    #[arg(short = 'w', long, alias = "proving-key-plonk")]
    proving_key_snark: Option<PathBuf>,

    /// Smaller STARK proof with reduced size. Mutually exclusive with --plonk and --pilfflonk
    #[arg(short = 'c', long, conflicts_with_all = ["plonk", "pilfflonk"])]
    minimal: bool,

    #[command(flatten)]
    snark: SnarkArgs,

    /// Output file path
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Use GPU acceleration
    #[cfg(not(feature = "cpu-only"))]
    #[arg(short = 'g', long)]
    gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl WrapCmd {
    pub(crate) fn run(&self) -> Result<()> {
        print_banner();

        print_banner_command("Wrap SNARK");

        let zisk_proof = Proof::load(&self.proof).map_err(|e| {
            anyhow::anyhow!("Failed to load Proof from file {}: {}", self.proof.display(), e)
        })?;

        let proof_kind = select_wrap_kind(self.snark.protocol(), self.minimal)?;
        if let Some(protocol) = self.snark.protocol() {
            ensure_snark_key(protocol, self.proving_key_snark.as_ref())?;
        }

        let mut prover_options = BackendProverOpts::default().verbose(self.verbose);
        if self.snark.protocol().is_some() {
            prover_options = prover_options.snark(true);
        }
        if let Some(ref path) = self.proving_key {
            prover_options = prover_options.proving_key(path.clone());
        }
        if let Some(ref path) = self.proving_key_snark {
            prover_options = prover_options.proving_key_snark(path.clone());
        }
        #[cfg(not(feature = "cpu-only"))]
        if self.gpu {
            prover_options = prover_options.gpu();
        }

        let prover =
            ProverClientBuilder::new().emu().with_prover_options(prover_options).build()?;

        let result = prover.wrap_proof(&zisk_proof, proof_kind).run()?;

        let output_file = resolve_output_path(self.output.clone(), None::<&str>);
        result.save_proof(&output_file).map_err(|e| {
            anyhow::anyhow!("Failed to save proof to {}: {}", output_file.display(), e)
        })?;

        info!("Final {} proof generated.", wrap_kind_label(proof_kind));

        Ok(())
    }
}
