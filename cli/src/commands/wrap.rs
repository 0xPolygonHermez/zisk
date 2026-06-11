use std::path::PathBuf;

use anyhow::Result;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::{Proof, ProofKind};
use zisk_prover_backend::{BackendProverOpts, ProverClientBuilder};

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a PLONK/minimal proof from a STARK (VADCOP) proof
pub struct ZiskWrap {
    /// Path to the STARK (VADCOP) proof file
    #[arg(short = 'p', long)]
    pub proof: PathBuf,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Path to a precomputed PLONK proving key
    #[arg(short = 'w', long)]
    pub proving_key_plonk: Option<PathBuf>,

    /// Smaller STARK proof with reduced size. Mutually exclusive with --plonk
    #[arg(short = 'c', long, conflicts_with = "plonk")]
    pub minimal: bool,

    /// PLONK proof for on-chain verification via the EVM verifier. Mutually exclusive with --minimal
    #[arg(long, conflicts_with = "minimal")]
    pub plonk: bool,

    /// Output file path
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Use GPU acceleration
    #[cfg(not(feature = "cpu-only"))]
    #[arg(short = 'g', long)]
    pub gpu: bool,

    /// Run mops planner on CPU even when --gpu is set (no-op for wrap, accepted for flag-parity)
    #[cfg(not(feature = "cpu-only"))]
    #[arg(long)]
    pub cpu_mops: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskWrap {
    pub fn run(&self) -> Result<()> {
        print_banner();

        print_banner_command("Wrap SNARK");

        let zisk_proof = Proof::load(&self.proof).map_err(|e| {
            anyhow::anyhow!("Failed to load Proof from file {}: {}", self.proof.display(), e)
        })?;

        let mut prover_options = BackendProverOpts::default().verbose(self.verbose);
        if self.plonk {
            prover_options = prover_options.plonk(true);
        }
        if let Some(ref path) = self.proving_key {
            prover_options = prover_options.proving_key(path.clone());
        }
        if let Some(ref path) = self.proving_key_plonk {
            prover_options = prover_options.proving_key_plonk(path.clone());
        }
        #[cfg(not(feature = "cpu-only"))]
        if self.gpu {
            prover_options = prover_options.gpu();
        }
        #[cfg(not(feature = "cpu-only"))]
        if self.cpu_mops {
            prover_options = prover_options.cpu_mops();
        }

        let prover =
            ProverClientBuilder::new().emu().with_prover_options(prover_options).build()?;

        let proof_kind = if self.plonk {
            ProofKind::Plonk
        } else if self.minimal {
            ProofKind::VadcopFinalMinimal
        } else {
            anyhow::bail!("Either --plonk or --minimal must be specified.");
        };

        let result = prover.wrap_proof(&zisk_proof, proof_kind).run()?;

        let output_file = self.output.clone().unwrap_or_else(|| {
            if self.plonk {
                PathBuf::from("vadcop_final_proof_plonk.bin")
            } else {
                PathBuf::from("vadcop_final_proof_minimal.bin")
            }
        });
        result.save_proof(&output_file).map_err(|e| {
            anyhow::anyhow!("Failed to save proof to {}: {}", output_file.display(), e)
        })?;

        let kind_label = if self.plonk { "PLONK" } else { "minimal" };
        info!("Final {} proof generated.", kind_label);

        Ok(())
    }
}
