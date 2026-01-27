use crate::ux::print_banner;
use anyhow::Result;

use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::ZiskStdin;
#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;
use zisk_sdk::{ProverClient, ZiskVerifyConstraintsResult};

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskVerifyConstraints {
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
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

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

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    #[clap(short = 'j', long, default_value_t = false)]
    pub shared_tables: bool,

    #[clap(short = 'm', long, default_value_t = false)]
    pub minimal_memory: bool,
}

impl ZiskVerifyConstraints {
    pub fn run(&mut self) -> Result<()> {
        print_banner();

        let stdin = self.create_stdin()?;

        let emulator = if cfg!(target_os = "macos") { true } else { self.emulator };
        let result = if emulator { self.run_emu(stdin)? } else { self.run_asm(stdin)? };

        tracing::info!("");
        tracing::info!(
            "{}",
            "--- VERIFY CONSTRAINTS SUMMARY ------------------------".bright_green().bold()
        );
        tracing::info!("    ► Statistics");
        tracing::info!(
            "      time: {:.2} seconds, steps: {}",
            result.duration.as_secs_f32(),
            result.execution.executed_steps
        );

        Ok(())
    }

    fn create_stdin(&mut self) -> Result<ZiskStdin> {
        let stdin = if let Some(input) = &self.input {
            if !input.exists() {
                return Err(anyhow::anyhow!("Input file not found at {:?}", input.display()));
            }
            ZiskStdin::from_file(input)?
        } else {
            ZiskStdin::null()
        };
        Ok(stdin)
    }

    pub fn run_emu(&mut self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult> {
        let prover = ProverClient::builder()
            .emu()
            .verify_constraints()
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .minimal_memory(self.minimal_memory)
            .print_command_info()
            .build()?;

        prover.verify_constraints_debug(stdin, self.debug.clone())
    }

    pub fn run_asm(&mut self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult> {
        let prover = ProverClient::builder()
            .asm()
            .verify_constraints()
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .asm_path_opt(self.asm.clone())
            .base_port_opt(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .minimal_memory(self.minimal_memory)
            .print_command_info()
            .build()?;

        prover.verify_constraints_debug(stdin, self.debug.clone())
    }
}
