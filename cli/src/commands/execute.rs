use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use tracing::{info, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{ProverClient, ZiskExecuteResult};

use crate::{
    commands::cli_fail_if_gpu_mode,
    ux::{print_banner, print_banner_field},
};
use zisk_common::io::{StreamSource, ZiskStdin};

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskExecute {
    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ROM file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// ASM file path
    /// Optional, mutually exclusive with `--emulator`
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Input path
    #[clap(short = 'i', long, alias = "input")]
    pub inputs: Option<String>,

    /// Precompiles Hints path
    #[clap(long)]
    pub hints: Option<String>,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Base port for Assembly microservices (default: 23115).
    /// A single execution will use 3 consecutive ports, from this port to port + 2.
    /// If you are running multiple instances of ZisK using mpi on the same machine,
    /// it will use from this base port to base port + 2 * number_of_instances.
    /// For example, if you run 2 mpi instances of ZisK, it will use ports from 23115 to 23117
    /// for the first instance, and from 23118 to 23120 for the second instance.
    #[clap(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    /// If you are running ZisK on a machine with limited memory, you may want to enable this option.
    /// This option is mutually exclusive with `--emulator`.
    #[clap(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'j', long, default_value_t = false)]
    pub shared_tables: bool,
}

impl ZiskExecute {
    pub fn run(&mut self) -> Result<()> {
        cli_fail_if_gpu_mode()?;

        // Check if the deprecated alias was used
        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        print_banner();

        if let Some(inputs) = &self.inputs {
            print_banner_field("Input", inputs);
        }

        if let Some(hints) = &self.hints {
            print_banner_field("Prec. Hints", hints);
        }

        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;

        let hints_stream = StreamSource::from_uri(self.hints.as_deref())?;

        let emulator = if cfg!(target_os = "macos") {
            if !self.emulator {
                warn!("Emulator mode is forced on macOS due to lack of ASM support.");
            }
            true
        } else {
            self.emulator
        };

        let result =
            if emulator { self.run_emu(stdin)? } else { self.run_asm(stdin, Some(hints_stream))? };

        info!("Execution completed in {:.2?}, steps: {}", result.duration, result.execution.steps);

        Ok(())
    }

    pub fn run_emu(&mut self, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        let prover = ProverClient::builder()
            .emu()
            .witness()
            .witness_lib_path_opt(self.witness_lib.clone())
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .print_command_info()
            .build()?;

        prover.execute(stdin, None)
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
    ) -> Result<ZiskExecuteResult> {
        let prover = ProverClient::builder()
            .asm()
            .verify_constraints()
            .witness_lib_path_opt(self.witness_lib.clone())
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .asm_path_opt(self.asm.clone())
            .base_port_opt(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .print_command_info()
            .build()?;

        prover.execute(stdin, hints_stream)
    }
}
