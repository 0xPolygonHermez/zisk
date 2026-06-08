use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{EmbeddedClientBuilder, Proof};

use crate::common::resolve_output_path;
use crate::proof::{select_wrap_kind, wrap_kind_label};
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a PLONK/minimal proof from a STARK (VADCOP) proof locally
pub(crate) struct ZiskEmbeddedWrap {
    /// Path to the STARK (VADCOP) proof file to wrap
    #[arg(short = 'p', long)]
    proof: PathBuf,

    /// Output file path
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    proving_key: Option<PathBuf>,

    /// Path to a precomputed PLONK proving key
    #[arg(short = 'w', long)]
    proving_key_plonk: Option<PathBuf>,

    /// Smaller STARK proof with reduced size. Mutually exclusive with --plonk
    #[arg(short = 'c', long, conflicts_with = "plonk")]
    minimal: bool,

    /// PLONK proof for on-chain verification via the EVM verifier. Mutually exclusive with --minimal
    #[arg(long, conflicts_with = "minimal")]
    plonk: bool,

    /// Use GPU acceleration
    #[cfg(not(feature = "cpu-only"))]
    #[arg(short = 'g', long)]
    gpu: bool,

    /// Verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskEmbeddedWrap {
    pub(crate) fn run(&mut self) -> Result<()> {
        print_banner();
        print_banner_command(format!("{} Wrap", "EMBEDDED".bold()));
        print_banner_field("Proof", self.proof.display());
        println!();

        let proof_kind = select_wrap_kind(self.plonk, self.minimal)?;
        let kind_label = wrap_kind_label(proof_kind);

        let zisk_proof = Proof::load(&self.proof).map_err(|e| {
            anyhow::anyhow!("Failed to load proof from {}: {}", self.proof.display(), e)
        })?;

        let mut builder = EmbeddedClientBuilder::default().verbose(self.verbose);
        if self.plonk {
            builder = builder.plonk();
        }
        #[cfg(not(feature = "cpu-only"))]
        if self.gpu {
            builder = builder.gpu();
        }
        if let Some(pk) = &self.proving_key {
            builder = builder.proving_key(pk.clone());
        }
        if let Some(pk) = &self.proving_key_plonk {
            builder = builder.proving_key_plonk(pk.clone());
        }
        let client = builder.build()?;

        // The embedded SDK exposes a synchronous path (`run_sync`) so no async
        // runtime is needed here.
        let result = client.wrap_proof(&zisk_proof, proof_kind).run_sync()?;

        let output_file = resolve_output_path(self.output.clone(), result.job_id());
        result.save_proof(&output_file).map_err(|e| {
            anyhow::anyhow!("Failed to save proof to {}: {}", output_file.display(), e)
        })?;

        info!("{}", "--- WRAP SUMMARY --------------".bright_green().bold());
        info!("Final {} proof saved to {}", kind_label, output_file.display());

        Ok(())
    }
}
