use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, Proof, RemoteClient};

use crate::common::resolve_output_path;
use crate::proof::{select_wrap_kind, wrap_kind_label};
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a PLONK/minimal proof from a STARK (VADCOP) proof on the remote service
pub(crate) struct ZiskRemoteWrap {
    /// Path to the STARK (VADCOP) proof file to wrap
    #[arg(short = 'p', long)]
    proof: PathBuf,

    /// Output file path
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Smaller STARK proof with reduced size. Mutually exclusive with --plonk
    #[arg(short = 'c', long, conflicts_with = "plonk")]
    minimal: bool,

    /// PLONK proof for on-chain verification via the EVM verifier. Mutually exclusive with --minimal
    #[arg(long, conflicts_with = "minimal")]
    plonk: bool,

    /// Wrap timeout in seconds (0 = no timeout)
    #[arg(long, default_value_t = 0)]
    timeout: u64,
}

impl ZiskRemoteWrap {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        print_banner();
        print_banner_command(format!("{} Wrap", "REMOTE".bold()));
        print_banner_field("Proof", self.proof.display());

        println!();

        setup_logger(zisk_sdk::VerboseMode::Info);

        let proof_kind = select_wrap_kind(self.plonk, self.minimal)?;
        let kind_label = wrap_kind_label(proof_kind);

        let zisk_proof = Proof::load(&self.proof).map_err(|e| {
            anyhow::anyhow!("Failed to load proof from {}: {}", self.proof.display(), e)
        })?;

        let mut request = client.wrap_proof(&zisk_proof, proof_kind);
        if self.timeout != 0 {
            request = request.timeout(Duration::from_secs(self.timeout));
        }
        let result = request.run()?.await?;

        let output_file = resolve_output_path(self.output.clone(), result.job_id());
        result.save_proof(&output_file).map_err(|e| {
            anyhow::anyhow!("Failed to save proof to {}: {}", output_file.display(), e)
        })?;

        info!("{}", "--- WRAP SUMMARY --------------".bright_green().bold());
        info!("Final {} proof saved to {}", kind_label, output_file.display());

        Ok(())
    }
}
