use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use tracing::{info, warn};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::ElfBinaryFromFile;
use zisk_sdk::{ProverClient, ZiskExecuteResult};

use crate::ux::{print_banner, print_banner_command, print_banner_field, print_execution_summary};
use super::{detect_current_project_elf, resolve_elf_path};
use zisk_common::io::{StreamSource, ZiskStdin};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Execute the guest program through the same pipeline that prove command uses but without generating a proof
pub struct ZiskExecute {
    /// Path to the program ELF file
    #[arg(short = 'e', long)]
    pub elf: Option<PathBuf>,

    //TODO: conflicts with --elf. Revisar si lo ponemos en 0.17.0
    /// Id of your program generated during setup
    #[arg(short = 'p', long, conflicts_with = "elf")]
    pub program_id: Option<String>,

    /// Use prebuilt emulator
    #[arg(short = 'l', long, conflicts_with = "asm")]
    pub emulator: bool,

    /// Input file path for the guest. Accepts a string literal or a path to a binary file
    #[arg(alias = "input",short = 'i', long, conflicts_with = "hints")]
    pub inputs: Option<String>,

    /// Save the input to the specified file path. Only used if `--inputs` is a string literal and not a file path
    #[arg(long, requires = "inputs")]
    pub save_inputs: bool,

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

    /// Verbose (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8, // Using u8 to hold the number of `-v`

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

    /// Use shared tables for execution
    #[arg(short = 'j', long, hide = true)]
    pub shared_tables: bool,
}

impl ZiskExecute {
    pub fn run(&mut self) -> Result<()> {
        if self.elf.is_none() && self.program_id.is_none() {
            self.elf = detect_current_project_elf()?;
        }

        // Check if the deprecated alias was used
        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        print_banner();

        print_banner_command("Execute");

        if let Some(elf) = &self.elf {
            print_banner_field("Elf", elf.display());
        } else if let Some(program_id) = &self.program_id {
            print_banner_field("Program ID", program_id);
        }

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
        let prover = ProverClient::builder()
            .emu()
            .witness()
            .proving_key_path_opt(self.proving_key.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .print_command_info()
            .build()?;

        let elf_path = resolve_elf_path(&self.elf)?;
        let elf = ElfBinaryFromFile::new(elf_path, false)?;
        let (pk, _) = prover.setup(&elf)?;
        prover.execute(&pk, stdin)
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
    ) -> Result<ZiskExecuteResult> {
        let prover = ProverClient::builder()
            .asm()
            .witness()
            .proving_key_path_opt(self.proving_key.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .asm_path_opt(self.asm.clone())
            .no_auto_setup(self.no_auto_setup)
            .base_port_opt(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .asm_out_file(self.asm_out_file)
            .print_command_info()
            .build()?;

        let elf_path = resolve_elf_path(&self.elf)?;
        let elf = ElfBinaryFromFile::new(elf_path, hints_stream.is_some())?;
        let (pk, _) = prover.setup(&elf)?;
        if let Some(hints_stream) = hints_stream {
            pk.register_hints_stream(hints_stream)?;
        }
        prover.execute(&pk, stdin)
    }
}
