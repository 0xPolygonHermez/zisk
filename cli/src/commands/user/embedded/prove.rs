use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{EmbeddedClientBuilder, GuestProgram, ProofKind, ZiskHints, ZiskStdin};

use super::{run_embedded_setup, validate_asm_hints};
use crate::common::{default_proof_filename, resolve_elf};
use crate::ux::print_job_banner;

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a proof for a guest program locally
pub(crate) struct ZiskEmbeddedProve {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,

    /// Input for the guest. Accepts a file path, `file://path`, or inline data
    /// `inline://[[1,2],[3]]` (a JSON array of u64 arrays, one frame per inner array)
    #[arg(short = 'i', long)]
    inputs: Option<String>,

    /// Precompiles hints URI for the guest. Requires the ASM backend (`--asm`).
    ///
    /// # URI Formats
    /// - `None` → null stream (no input)
    /// - `"scheme://resource"` → parsed based on scheme
    /// - No scheme → treated as a file path
    ///
    /// # Supported Schemes
    /// - `file://path/to/file`   → File-based stream
    /// - `unix://path/to/socket` → Unix domain socket stream
    #[arg(long, conflicts_with = "inputs")]
    hints: Option<String>,

    /// Use the ASM emulator instead of the default Rust emulator
    #[arg(short = 'a', long)]
    asm: bool,

    /// Save the generated proof to the specified file path
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Smaller STARK proof with reduced size at the cost of longer proving time. Mutually exclusive with --plonk
    #[arg(short = 'c', long, conflicts_with = "plonk")]
    minimal: bool,

    /// PLONK proof. Required for on-chain verification via the EVM verifier. Mutually exclusive with --minimal
    #[arg(long, conflicts_with = "minimal")]
    plonk: bool,

    /// Verify the proof after generation
    #[arg(short = 'y', long)]
    verify_proof: bool,

    /// Verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskEmbeddedProve {
    pub(crate) fn run(&mut self) -> Result<()> {
        let elf = resolve_elf(self.elf.take())?;
        validate_asm_hints(self.asm, self.hints.as_deref())?;

        print_job_banner(
            &format!("{} Prove", "EMBEDDED".bold()),
            &elf,
            self.inputs.as_deref(),
            self.hints.as_deref(),
        );
        println!();

        let program = GuestProgram::from_uri(elf.to_str().unwrap())?;
        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;
        let hints = self.hints.as_ref().map(ZiskHints::from_uri).transpose()?;

        // VadcopFinal by default; --minimal / --plonk select a wrapped proof.
        let proof_kind = if self.plonk {
            ProofKind::Plonk
        } else if self.minimal {
            ProofKind::VadcopFinalMinimal
        } else {
            ProofKind::VadcopFinal
        };

        let mut builder = EmbeddedClientBuilder::default().verbose(self.verbose);
        if self.asm {
            builder = builder.assembly();
        }
        if self.plonk {
            builder = builder.plonk();
        }
        let client = builder.build()?;

        run_embedded_setup(&client, &program, self.asm, hints.is_some())?;

        let mut request = client.prove(&program, stdin);
        if let Some(hints) = hints {
            request = request.hints(hints);
        }
        if proof_kind != ProofKind::VadcopFinal {
            request = request.wrap(proof_kind);
        }
        let result = request.run_sync()?;

        let output_file = self.output.clone().unwrap_or(default_proof_filename(result.job_id()));
        result.save_proof(&output_file).map_err(|e| {
            anyhow::anyhow!("Failed to save proof to {}: {}", output_file.display(), e)
        })?;

        if self.verify_proof {
            result.verify()?;
            info!("{}", "Proof verified successfully.".bright_green());
        }

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
