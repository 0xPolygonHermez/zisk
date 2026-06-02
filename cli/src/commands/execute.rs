use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use tracing::{info, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::{
    AsmOptions, BackendProverOpts, ExecuteClient, ExecuteOutput, GuestProgram, ProverClientBuilder,
};

use crate::common::detect_current_project_elf;
use crate::ux::{print_banner, print_banner_command, print_banner_field, print_execution_summary};
use zisk_common::io::{StreamSource, ZiskStdin};

/// Rank-0 check for output gating under `mpirun`. Defaults to true when the env
/// var is absent so non-MPI runs print normally.
fn is_rank_zero() -> bool {
    std::env::var("OMPI_COMM_WORLD_RANK").map(|s| s == "0").unwrap_or(true)
}

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Execute the guest program. Defaults to the full prover path (loads proving
/// key, supports MPI under `mpirun`). Pass `--standalone` for a fast no-keys
/// run intended for dev iteration.
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

    /// Precompiles hints file path for the guest
    #[arg(long, conflicts_with = "inputs")]
    pub hints: Option<String>,

    /// This is used to unlock the memory map for the ROM file. Mutually exclusive with --emulator
    #[arg(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Skip loading proving keys. Single-process only — no MPI. Faster startup
    /// for dev iteration and cargo tests; prints a plan summary.
    #[arg(long, conflicts_with = "proving_key")]
    pub standalone: bool,

    /// Run the memory-ops count-and-plan on the GPU. On `--standalone` the
    /// executor self-allocates its own device arena; on the proofman path it
    /// borrows proofman's unified buffer. Requires a CUDA build and a usable
    /// GPU; falls back to the CPU planner otherwise. Defaults to CPU.
    #[arg(long)]
    pub gpu: bool,

    /// Path to the proving key. Defaults to the standard install location.
    /// Ignored under `--standalone`.
    #[arg(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    // Hidden flags
    /// ASM file path
    #[arg(short = 's', long, hide = true, conflicts_with = "emulator")]
    pub asm: Option<PathBuf>,

    /// Redirect ASM emulator output to file
    #[arg(long, conflicts_with = "emulator", hide = true)]
    pub asm_out_file: bool,
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

        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        let rank_zero = is_rank_zero();

        if rank_zero {
            print_banner();
            print_banner_command("Execute");
            print_banner_field("Elf", self.elf.as_ref().unwrap().display());

            let inputs_str = self.inputs.clone().unwrap_or_else(|| "None".dimmed().to_string());
            print_banner_field("Input", inputs_str);

            if let Some(hints) = &self.hints {
                print_banner_field("Prec. Hints", hints);
            }
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
            if !self.emulator && rank_zero {
                warn!("Emulator mode is forced on macOS due to lack of ASM support.");
            }
            true
        } else {
            self.emulator
        };

        let guest_program = GuestProgram::from_uri(self.elf.as_ref().unwrap().to_str().unwrap())?;
        let prover_options = self.make_prover_options();
        let with_hints = hints_stream.is_some();

        let prover: Box<dyn ExecuteClient> = match (self.standalone, emulator) {
            (true, true) => Box::new(
                ProverClientBuilder::new()
                    .emu()
                    .with_prover_options(prover_options)
                    .execute_only()
                    .build()?,
            ),
            (true, false) => Box::new(
                ProverClientBuilder::new()
                    .asm()
                    .with_prover_options(prover_options.with_asm_options(self.make_asm_options()))
                    .execute_only()
                    .build()?,
            ),
            (false, true) => Box::new(
                ProverClientBuilder::new().emu().with_prover_options(prover_options).build()?,
            ),
            (false, false) => Box::new(
                ProverClientBuilder::new()
                    .asm()
                    .with_prover_options(prover_options.with_asm_options(self.make_asm_options()))
                    .build()?,
            ),
        };

        let start = std::time::Instant::now();
        prover.setup(&guest_program, with_hints)?;
        let result: ExecuteOutput = prover.execute(&guest_program, stdin, hints_stream)?;
        let total_duration = start.elapsed().as_millis() as u64;

        if rank_zero {
            if let Some(plan) = result.get_plan() {
                let total: usize = plan.iter().map(|e| e.count).sum();
                info!("{}", "--- PLAN SUMMARY --------------".bright_green().bold());
                use std::collections::BTreeMap;
                let mut by_group: BTreeMap<usize, Vec<&zisk_prover_backend::PlanSummaryEntry>> =
                    BTreeMap::new();
                for entry in plan {
                    by_group.entry(entry.airgroup_id).or_default().push(entry);
                }
                for (airgroup_id, entries) in &by_group {
                    let group_name = if *airgroup_id == 0 { "Zisk" } else { "Unknown" };
                    let parts: Vec<String> =
                        entries.iter().map(|e| format!("{}: {}", e.name, e.count)).collect();
                    info!(
                        "{} | {} | Total instances: {}",
                        group_name.bright_white().bold(),
                        parts.join(" | "),
                        total
                    );
                }
            }

            info!("{}", "--- EXECUTE SUMMARY -----------".bright_green().bold());
            print_execution_summary(
                result.get_executor_time(),
                total_duration,
                result.get_execution_steps(),
                if self.standalone { "Setup" } else { "Proofman" },
            );
        }

        Ok(())
    }

    fn make_prover_options(&self) -> BackendProverOpts {
        let mut opts = BackendProverOpts::default().verbose(self.verbose);
        if let Some(ref path) = self.proving_key {
            opts = opts.proving_key(path.clone());
        }
        if self.gpu {
            opts = opts.gpu();
        }
        opts
    }

    fn make_asm_options(&self) -> AsmOptions {
        let mut a = AsmOptions::default();
        if let Some(ref path) = self.asm {
            a = a.asm_path(path.clone());
        }
        if self.unlock_mapped_memory {
            a = a.unlock_mapped_memory();
        }
        if self.asm_out_file {
            a = a.asm_out_file();
        }
        a
    }
}
