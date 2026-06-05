use crate::common::detect_current_project_elf;
use crate::ux::{print_banner, print_banner_command, print_banner_field, print_execution_summary};
use anyhow::Result;

use colored::Colorize;
use std::path::PathBuf;
use tracing::{info, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::{ProofKind, ZiskExecutorTime};
use zisk_prover_backend::GuestProgram;
use zisk_prover_backend::{AsmOptions, BackendProverOpts, ProveOutput, ProverClientBuilder};

// Structure representing the 'prove' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a proof from the execution of the guest program
pub struct ZiskProve {
    /// Path to the program ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    pub elf: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[arg(short = 'l', long, conflicts_with = "asm")]
    pub emulator: bool,

    /// Input file path for the guest. Accepts a string literal or a path to a binary file
    #[arg(alias = "input", short = 'i', long, conflicts_with = "hints")]
    pub inputs: Option<String>,

    // Save the input to the specified file path. Only used if `--inputs` is a string literal and not a file path
    // #[arg(long, requires = "inputs")]
    // pub save_inputs: bool,
    //
    /// Precompiles hints file path for the guest
    #[arg(long, conflicts_with = "inputs")]
    pub hints: Option<String>,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Path to a precomputed PLONK proving key
    #[arg(short = 'w', long)]
    pub proving_key_plonk: Option<PathBuf>,

    /// Save the generated proof to the specified file path
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Disable proofs aggregation
    #[arg(short = 'a', long, default_value_t = false)]
    pub no_aggregation: bool,

    /// Smaller STARK proof with reduced size at the cost of longer proving time. Mutually exclusive with plonk
    #[arg(short = 'c', long, conflicts_with = "plonk")]
    pub minimal: bool,

    /// PLONK proof. Required for on-chain verification via the EVM verifier. Mutually exclusive with minimal
    #[arg(long, conflicts_with = "minimal")]
    pub plonk: bool,

    /// Verify proofs after generation
    #[arg(short = 'y', long)]
    pub verify_proofs: bool,

    /// This is used to unlock the memory map for the ROM file. Mutually exclusive with --emulator
    #[arg(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Maximum memory (bytes) for witness storage during proving
    #[arg(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    /// Reduce memory footprint during proving at the cost of speed
    #[arg(short = 'm', long)]
    pub minimal_memory: bool,

    /// Use GPU acceleration
    #[cfg(not(feature = "cpu-only"))]
    #[arg(short = 'g', long)]
    pub gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    // Hidden flags
    /// ASM file path
    #[arg(short = 's', long, hide = true, conflicts_with = "emulator")]
    pub asm: Option<PathBuf>,

    /// Redirect ASM emulator output to file
    #[arg(long, hide = true, conflicts_with = "emulator")]
    pub asm_out_file: bool,

    /// Disable automatic ROM setup
    #[arg(short = 'n', long, hide = true)]
    pub no_auto_setup: bool,

    #[arg(short = 'z', long, default_value_t = false, hide = true)]
    pub preallocate_fixed_gpu: bool,

    /// Maximum number of concurrent GPU streams for proving
    #[arg(short = 't', long, hide = true)]
    pub max_streams: Option<usize>,

    /// Number of threads per worker pool used during witness computation
    #[arg(long, hide = true)]
    pub number_threads_witness: Option<usize>,
}

impl ZiskProve {
    pub fn run(&mut self) -> Result<()> {
        if self.elf.is_none() {
            self.elf = match detect_current_project_elf()? {
                Some(elf) => Some(elf),
                None => {
                    anyhow::bail!(
                        "No ELF file provided, and could not detect a project ELF in the current directory. Please provide an ELF file with --elf."
                    );
                }
            };
        }

        // Check if the deprecated alias was used
        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        print_banner();

        print_banner_command("Prove");

        print_banner_field("Elf", self.elf.as_ref().unwrap().display());

        let inputs_str = self.inputs.clone().unwrap_or_else(|| "None".dimmed().to_string());
        print_banner_field("Input", inputs_str);

        if let Some(hints) = &self.hints {
            print_banner_field("Prec. Hints", hints);
        }

        if self.plonk && self.minimal {
            anyhow::bail!("Minimal proofs are not supported for SNARK generation.");
        }

        // Build BackendProverOpts once with all configuration
        let mut prover_options =
            BackendProverOpts::default().aggregation(!self.no_aggregation).verbose(self.verbose);

        if self.minimal_memory {
            prover_options = prover_options.minimal_memory();
        }
        if self.verify_proofs {
            prover_options = prover_options.verify_proofs();
        }
        #[cfg(not(feature = "cpu-only"))]
        if self.gpu {
            prover_options = prover_options.gpu();
        }
        if self.plonk {
            prover_options = prover_options.plonk(false);
        }
        if let Some(ref path) = self.proving_key {
            prover_options = prover_options.proving_key(path.clone());
        }
        if let Some(ref path) = self.proving_key_plonk {
            prover_options = prover_options.proving_key_plonk(path.clone());
        }
        if let Some(max) = self.max_witness_stored {
            prover_options = prover_options.max_witness_stored(max);
        }
        if let Some(threads) = self.number_threads_witness {
            prover_options = prover_options.number_threads_witness(threads);
        }
        if let Some(max) = self.max_streams {
            prover_options = prover_options.max_streams(max);
        }

        // ASM-specific options (only used if not emulator)
        let mut asm_options = AsmOptions::default();
        if let Some(ref path) = self.asm {
            asm_options = asm_options.asm_path(path.clone());
        }
        if self.no_auto_setup {
            asm_options = asm_options.no_auto_setup();
        }
        if self.unlock_mapped_memory {
            asm_options = asm_options.unlock_mapped_memory();
        }
        if self.asm_out_file {
            asm_options = asm_options.asm_out_file();
        }
        prover_options = prover_options.with_asm_options(asm_options);

        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;

        let hints_stream = match self.hints.as_ref() {
            Some(uri) => {
                let stream = StreamSource::from_uri(uri)?;
                if matches!(stream, StreamSource::Quic(_)) {
                    anyhow::bail!("QUIC hints source is not supported in CLI mode.");
                }
                Some(stream)
            }
            None => None,
        };

        let emulator = if cfg!(target_os = "macos") {
            if !self.emulator {
                warn!("Emulator mode is forced on macOS due to lack of ASM support.");
            }
            true
        } else {
            self.emulator
        };

        let (result, executor_time) = if emulator {
            self.run_emu(stdin, prover_options)?
        } else {
            self.run_asm(stdin, hints_stream, prover_options)?
        };

        if !result.get_proof().is_empty() {
            info!("{}", "--- PROVE SUMMARY ------------------------".bright_green().bold());

            let output_file: PathBuf = match result.get_proof().kind() {
                ProofKind::VadcopFinal | ProofKind::VadcopFinalMinimal => {
                    self.output.clone().unwrap_or_else(|| PathBuf::from("vadcop_final_proof.bin"))
                }
                ProofKind::Plonk => {
                    self.output.clone().unwrap_or_else(|| PathBuf::from("final_plonk_proof.bin"))
                }
            };
            result.save_proof(&output_file)?;
            info!("Proof Time: {:.3} seconds", result.get_proving_time() as f64 / 1000.0);

            print_execution_summary(
                &executor_time,
                result.get_proving_time(),
                result.get_execution_steps(),
                "Proofman",
            );
        }

        Ok(())
    }

    pub fn run_emu(
        &mut self,
        stdin: ZiskStdin,
        prover_options: BackendProverOpts,
    ) -> Result<(ProveOutput, ZiskExecutorTime)> {
        let prover =
            ProverClientBuilder::new().emu().with_prover_options(prover_options).build()?;

        let guest_program = GuestProgram::from_uri(self.elf.as_ref().unwrap().to_str().unwrap())?;
        prover.setup(&guest_program).run()?;

        let mut builder = prover.prove(&guest_program, stdin);
        if self.plonk {
            builder = builder.wrap_proof(ProofKind::Plonk);
        }
        if self.minimal {
            builder = builder.wrap_proof(ProofKind::VadcopFinalMinimal);
        }
        let result = builder.run()?;
        let executor_time = prover.get_executor_time()?;

        Ok((result, executor_time))
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        prover_options: BackendProverOpts,
    ) -> Result<(ProveOutput, ZiskExecutorTime)> {
        let prover =
            ProverClientBuilder::new().asm().with_prover_options(prover_options).build()?;

        let guest_program = GuestProgram::from_uri(self.elf.as_ref().unwrap().to_str().unwrap())?;
        if hints_stream.is_some() {
            prover.setup(&guest_program).with_hints().run()?;
        } else {
            prover.setup(&guest_program).run()?;
        }

        if let Some(hints_stream) = hints_stream {
            prover.register_hints_stream(hints_stream)?;
        }

        let mut builder = prover.prove(&guest_program, stdin);
        if self.plonk {
            builder = builder.wrap_proof(ProofKind::Plonk);
        }
        if self.minimal {
            builder = builder.wrap_proof(ProofKind::VadcopFinalMinimal);
        }

        let result = builder.run()?;
        let executor_time = prover.get_executor_time()?;

        Ok((result, executor_time))
    }
}
