use crate::{
    commands::{cli_fail_if_gpu_mode, Field},
    ux::print_banner,
    ZISK_VERSION_MESSAGE,
};
use anyhow::Result;

use clap::Parser;
use colored::Colorize;
use std::{path::PathBuf, time::Duration};
#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;
use zisk_common::ZiskExecutionResult;
use zisk_sdk::ProverClient;

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
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

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
}

impl ZiskVerifyConstraints {
    pub fn run(&mut self) -> Result<()> {
        cli_fail_if_gpu_mode()?;

        print_banner();

        let (result, elapsed) = if self.emulator { self.run_emu()? } else { self.run_asm()? };

        tracing::info!("");
        tracing::info!(
            "{}",
            "--- VERIFY CONSTRAINTS SUMMARY ------------------------".bright_green().bold()
        );
        tracing::info!("    ► Statistics");
        tracing::info!(
            "      time: {} seconds, steps: {}",
            elapsed.as_secs_f32(),
            result.executed_steps
        );

        // Store the stats in stats.json
        #[cfg(feature = "stats")]
        {
            let stats_id = _stats.next_id();
            _stats.add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
            _stats.store_stats();
        }

        Ok(())
    }

    pub fn run_emu(&mut self) -> Result<(ZiskExecutionResult, Duration)> {
        let prover = ProverClient::builder()
            .emu()
            .verify_constraints()
            .witness_lib_path(self.witness_lib.clone())
            .proving_key_path(self.proving_key.clone())
            .elf_path(Some(self.elf.clone()))
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .print_command_info()
            .build()?;

        let start = std::time::Instant::now();
        prover.debug_verify_constraints(self.input.clone(), self.debug.clone())?;
        let elapsed = start.elapsed();

        let (result, mut _stats) = prover.execution_result().ok_or_else(|| {
            anyhow::anyhow!("Failed to get execution result from emulator prover")
        })?;

        Ok((result, elapsed))
    }

    pub fn run_asm(&mut self) -> Result<(ZiskExecutionResult, Duration)> {
        let prover = ProverClient::builder()
            .asm()
            .verify_constraints()
            .witness_lib_path(self.witness_lib.clone())
            .proving_key_path(self.proving_key.clone())
            .elf_path(Some(self.elf.clone()))
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .asm_path(self.asm.clone())
            .base_port(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .print_command_info()
            .build()?;

        let start = std::time::Instant::now();
        prover.debug_verify_constraints(self.input.clone(), self.debug.clone())?;
        let elapsed = start.elapsed();

        let (result, mut _stats) = prover
            .execution_result()
            .ok_or_else(|| anyhow::anyhow!("Failed to get execution result from ASM prover"))?;

        Ok((result, elapsed))
    }
}
