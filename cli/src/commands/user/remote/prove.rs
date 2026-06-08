use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, GuestProgram, ProofKind, RemoteClient, ZiskHints, ZiskStdin};

use crate::common::{reject_quic_hints, resolve_elf, resolve_output_path, ProfileArgs};
use crate::proof::select_prove_kind;
use crate::ux::print_job_banner;

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a proof for a guest program on the remote service
///
/// The program must already be registered and set up (run `remote setup` first).
pub(crate) struct ZiskRemoteProve {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,

    #[command(flatten)]
    profile: ProfileArgs,

    /// Input for the guest. Accepts a file path, `file://path`, or inline data
    /// `inline://[[1,2],[3]]` (a JSON array of u64 arrays, one frame per inner array)
    #[arg(short = 'i', long)]
    inputs: Option<String>,

    /// Precompiles hints URI for the guest (sent inline to the coordinator).
    ///
    /// `file://path` or a plain path is read and sent inline. `quic://` is not
    /// supported from the CLI.
    #[arg(long, conflicts_with = "inputs")]
    hints: Option<String>,

    /// Save the generated proof to the specified file path
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Smaller STARK proof with reduced size at the cost of longer proving time. Mutually exclusive with --plonk
    #[arg(short = 'c', long, conflicts_with = "plonk")]
    minimal: bool,

    /// PLONK proof. Required for on-chain verification via the EVM verifier. Mutually exclusive with --minimal
    #[arg(long, conflicts_with = "minimal")]
    plonk: bool,

    /// Proof timeout in seconds (0 = no timeout)
    #[arg(long, default_value_t = 0)]
    timeout: u64,
}

impl ZiskRemoteProve {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        let elf = resolve_elf(self.elf.take(), self.profile.profile())?;
        reject_quic_hints(self.hints.as_deref())?;

        print_job_banner(
            &format!("{} Prove", "REMOTE".bold()),
            &elf,
            self.inputs.as_deref(),
            self.hints.as_deref(),
        );

        println!();

        setup_logger(zisk_sdk::VerboseMode::Info);

        let program = GuestProgram::from_uri(elf.to_str().unwrap())?;
        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;
        let hints = self.hints.as_ref().map(ZiskHints::from_uri).transpose()?;

        // VadcopFinal by default; --minimal / --plonk select a wrapped proof.
        let proof_kind = select_prove_kind(self.plonk, self.minimal);

        let mut request = client.prove(&program, stdin);
        if let Some(hints) = hints {
            request = request.hints(hints);
        }
        if proof_kind != ProofKind::VadcopFinal {
            request = request.wrap(proof_kind);
        }
        if self.timeout != 0 {
            request = request.timeout(Duration::from_secs(self.timeout));
        }
        let result = request.run()?.await?;

        let output_file = resolve_output_path(self.output.clone(), result.job_id());
        result.save_proof(&output_file).map_err(|e| {
            anyhow::anyhow!("Failed to save proof to {}: {}", output_file.display(), e)
        })?;

        info!("{}", "--- PROVE SUMMARY -------------".bright_green().bold());
        info!(
            "Proof generated in {:.3}s, steps: {}",
            result.get_proving_time() as f64 / 1000.0,
            result.get_execution_steps()
        );
        info!("Proof saved to {}", output_file.display());

        Ok(())
    }
}
