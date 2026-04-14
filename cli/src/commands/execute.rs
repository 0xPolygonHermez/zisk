use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use tracing::{info, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::GuestProgram;
use zisk_prover_backend::{AsmOptions, BackendProverOpts, ProverClientBuilder, ZiskExecuteResult};

use crate::common::detect_current_project_elf;
use crate::ux::{print_banner, print_banner_command, print_banner_field, print_execution_summary};
use zisk_common::io::{StreamSource, ZiskStdin};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Execute the guest program through the same pipeline that prove command uses but without generating a proof
pub struct ZiskExecute {
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

    /// Base port for Assembly microservices (default: 23115).
    /// A single execution will use 3 consecutive ports, from this port to port + 2.
    /// If you are running multiple instances of ZisK using mpi on the same machine,
    /// it will use from this base port to base port + 2 * number_of_instances.
    /// For example, if you run 2 mpi instances of ZisK, it will use ports from 23115 to 23117
    /// for the first instance, and from 23118 to 23120 for the second instance.
    //TODO: Remove
    #[arg(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// This is used to unlock the memory map for the ROM file. Mutually exclusive with --emulator
    #[arg(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    // Hidden flags
    /// ASM file path
    #[arg(short = 's', long, hide = true, conflicts_with = "emulator")]
    pub asm: Option<PathBuf>,

    /// Redirect ASM emulator output to file
    #[arg(long, conflicts_with = "emulator", hide = true)]
    pub asm_out_file: bool,

    /// Disable automatic ROM setup
    #[arg(short = 'n', long, hide = true)]
    pub no_auto_setup: bool,
}

impl ZiskExecute {
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

        print_banner_command("Execute");
        print_banner_field("Elf", self.elf.as_ref().unwrap().display());

        let inputs_str = self.inputs.clone().unwrap_or_else(|| "None".dimmed().to_string());
        print_banner_field("Input", inputs_str);

        if let Some(hints) = &self.hints {
            print_banner_field("Prec. Hints", hints);
        }

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

        let result =
            if emulator { self.run_emu(stdin)? } else { self.run_asm(stdin, hints_stream)? };

        info!("{}", "--- EXECUTE SUMMARY ------------------------".bright_green().bold());
        print_execution_summary(
            &result.executor_summary.executor_time,
            result.total_duration,
            result.executor_summary.steps,
        );

        Ok(())
    }

    pub fn run_emu(&mut self, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        let mut prover_options = BackendProverOpts::default().verbose(self.verbose);

        if let Some(ref path) = self.proving_key {
            prover_options = prover_options.proving_key(path.clone());
        }

        let prover = ProverClientBuilder::new()
            .emu()
            .witness()
            .with_prover_options(prover_options)
            .build()?;

        let guest_program = GuestProgram::from_uri(self.elf.as_ref().unwrap().to_str().unwrap())?;
        prover.setup(&guest_program).run()?;
        prover.execute(&guest_program, stdin)
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
    ) -> Result<ZiskExecuteResult> {
        let mut prover_options = BackendProverOpts::default().verbose(self.verbose);

        if let Some(ref path) = self.proving_key {
            prover_options = prover_options.proving_key(path.clone());
        }

        // ASM-specific options (only used if not emulator)
        let mut asm_options = AsmOptions::default();
        if let Some(ref path) = self.asm {
            asm_options = asm_options.asm_path(path.clone());
        }
        if let Some(port) = self.port {
            asm_options = asm_options.base_port(port);
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

        let prover = ProverClientBuilder::new()
            .asm()
            .witness()
            .with_prover_options(prover_options)
            .build()?;

        let guest_program = GuestProgram::from_uri(self.elf.as_ref().unwrap().to_str().unwrap())?;
        if hints_stream.is_some() {
            prover.setup(&guest_program).with_hints().run()?;
        } else {
            prover.setup(&guest_program).run()?;
        }
        if let Some(hints_stream) = hints_stream {
            prover.register_hints_stream(hints_stream)?;
        }
        prover.execute(&guest_program, stdin)
    }
}
