use crate::ux::{print_banner, print_banner_command, print_banner_field, print_execution_summary};
use anyhow::Result;

use clap::Parser;
use colored::Colorize;
use executor::get_packed_info;
use proofman_common::ProofmanOptions;
use std::path::PathBuf;
use tracing::{info, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_prover_backend::GuestProgram;
use zisk_prover_backend::{ProverClientBuilder, ZiskVerifyConstraintsResult};

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Verify the constraints of the guest program execution without generating a proof
pub struct ZiskVerifyConstraints {
    /// Path to the program ELF file
    // TODO: Optional?
    #[arg(short = 'e', long)]
    pub elf: PathBuf,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[arg(short = 'l', long, conflicts_with = "asm")]
    pub emulator: bool,

    /// Input file path for the guest. Accepts a string literal or a path to a binary file
    #[arg(alias = "input", short = 'i', long, conflicts_with = "hints")]
    pub inputs: Option<String>,

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
    #[clap(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// This is used to unlock the memory map for the ROM file. Mutually exclusive with --emulator
    #[arg(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Use GPU acceleration
    #[clap(short = 'g', long, default_value_t = false)]
    pub gpu: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    // Hidden flags
    /// ASM file path
    #[arg(short = 's', long, hide = true, conflicts_with = "emulator")]
    pub asm: Option<PathBuf>,

    /// Redirect ASM emulator output to file
    #[arg(long, default_value_t = false, hide = true, conflicts_with = "emulator")]
    pub asm_out_file: bool,

    /// Disable automatic ROM setup
    #[arg(short = 'n', long, default_value_t = false, hide = true)]
    pub no_auto_setup: bool,

    /// Use shared tables for execution
    #[arg(short = 'j', long, default_value_t = false, hide = true)]
    pub no_shared_tables_mpi: bool,

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,
}

impl ZiskVerifyConstraints {
    pub fn run(&mut self) -> Result<()> {
        // panic::set_hook(Box::new(|panic_info| {
        //     eprintln!("\x1B[31mPANIC DETECTED");
        //     eprintln!("{} at {:?}", panic_info, panic_info.location());

        //     // Backtrace
        //     let bt = std::backtrace::Backtrace::force_capture();
        //     eprintln!("Backtrace:\n{}", bt);

        //     std::process::exit(101);
        // }));

        // Check if the deprecated alias was used
        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        print_banner();

        print_banner_command("Verify Constraints");

        print_banner_field("Elf", self.elf.display());

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

        let mut options = ProofmanOptions::default();
        options.verify_constraints();
        options.verbose_mode(self.verbose.into());
        if self.gpu {
            options.gpu();
        }
        options.packed_info(get_packed_info());

        let result = if emulator {
            self.run_emu(stdin, options)?
        } else {
            self.run_asm(stdin, hints_stream, options)?
        };

        info!(
            "{}",
            "--- VERIFY CONSTRAINTS SUMMARY ------------------------".bright_green().bold()
        );
        print_execution_summary(
            &result.executor_summary.executor_time,
            result.duration,
            result.executor_summary.steps,
        );

        Ok(())
    }

    pub fn run_emu(
        &mut self,
        stdin: ZiskStdin,
        options: ProofmanOptions,
    ) -> Result<ZiskVerifyConstraintsResult> {
        let prover = ProverClientBuilder::new()
            .emu()
            .verify_constraints()
            .proving_key_path_opt(self.proving_key.clone())
            .verbose(self.verbose)
            .shared_tables(!self.no_shared_tables_mpi)
            .options(options)
            .print_command_info()
            .build()?;

        let guest_program = GuestProgram::from_uri(self.elf.to_str().unwrap())?;
        prover.setup(&guest_program).run()?;

        prover.verify_constraints(&guest_program, stdin, self.debug.clone())
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
        options: ProofmanOptions,
    ) -> Result<ZiskVerifyConstraintsResult> {
        let prover = ProverClientBuilder::new()
            .asm()
            .verify_constraints()
            .proving_key_path_opt(self.proving_key.clone())
            .verbose(self.verbose)
            .shared_tables(!self.no_shared_tables_mpi)
            .asm_path_opt(self.asm.clone())
            .no_auto_setup(self.no_auto_setup)
            .base_port_opt(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .asm_out_file(self.asm_out_file)
            .options(options)
            .print_command_info()
            .build()?;

        let guest_program = GuestProgram::from_uri(self.elf.to_str().unwrap())?;
        if hints_stream.is_some() {
            prover.setup(&guest_program).with_hints().run()?;
        } else {
            prover.setup(&guest_program).run()?;
        }

        if let Some(hints_stream) = hints_stream {
            prover.register_hints_stream(hints_stream)?;
        }
        prover.verify_constraints(&guest_program, stdin, self.debug.clone())
    }
}
