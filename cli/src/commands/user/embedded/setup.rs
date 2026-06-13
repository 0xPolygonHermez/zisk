use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, AsmOptions, EmbeddedClientBuilder, GuestProgram, VerboseMode};

use super::validate_setup_asm;
use crate::commands::user::recurser_common::resolve_recurser;
use crate::common::{resolve_elf, ElfSelectorArgs, Profile};
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate the proving setup for a guest or aggregation program locally
pub(crate) struct ZiskEmbeddedSetup {
    /// Path to the guest ELF file. If omitted (and no `--aggregation`), the ELF
    /// is auto-detected from the current project
    #[arg(short = 'e', long, conflicts_with = "aggregation")]
    elf: Option<PathBuf>,

    /// Aggregation definition (`<programs>/aggregations/<name>.toml`) to set up
    /// as a recurser instead of a guest ELF. The referenced guest programs must
    /// already be built (`cargo build` of the host crate). Artifacts go to the
    /// SDK-managed `~/.zisk/recurser/<recurser-id>`.
    #[arg(
        long,
        conflicts_with_all = ["bin", "asm", "hints", "proving_key_plonk", "unlock_mapped_memory"]
    )]
    aggregation: Option<PathBuf>,

    #[command(flatten)]
    selector: ElfSelectorArgs,

    /// Use the ASM emulator instead of the default Rust emulator
    #[arg(short = 'a', long)]
    asm: bool,

    /// Enable precompiles hints support for this program. Requires the ASM backend (`--asm`).
    #[arg(long, requires = "asm")]
    hints: bool,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long)]
    proving_key: Option<PathBuf>,

    /// Path to a precomputed PLONK proving key
    #[arg(short = 'w', long)]
    proving_key_plonk: Option<PathBuf>,

    /// Unlock the memory map for the ROM file. Only applies with `--asm`.
    #[arg(short = 'u', long, requires = "asm")]
    unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskEmbeddedSetup {
    pub(crate) fn run(&mut self) -> Result<()> {
        if let Some(aggregation) = self.aggregation.take() {
            print_banner();
            print_banner_command(format!("{} Setup", "EMBEDDED".bold()));
            print_banner_field("Aggregation", aggregation.display());
            println!();

            setup_logger(VerboseMode::from(self.verbose));

            let agg = resolve_recurser(&aggregation, self.selector.profile() == Profile::Release)?;
            info!("Recurser ID: {}", agg.recurser_id());

            let mut builder = EmbeddedClientBuilder::default().verbose(self.verbose);
            if let Some(pk) = &self.proving_key {
                builder = builder.proving_key(pk.clone());
            }
            let client = builder.build()?;

            client.setup(&agg).run_sync()?;

            info!("{}", "--- SETUP SUMMARY -------------".bright_green().bold());
            info!("Setup completed for recurser ID: {}", agg.recurser_id());
            return Ok(());
        }

        let elf = resolve_elf(self.elf.take(), self.selector.profile(), self.selector.bin())?;
        validate_setup_asm(self.asm)?;

        print_banner();
        print_banner_command(format!("{} Setup", "EMBEDDED".bold()));
        print_banner_field("Elf", elf.display());
        println!();

        setup_logger(VerboseMode::from(self.verbose));

        let program = GuestProgram::from_uri(elf.to_str().unwrap())?;

        let mut builder = EmbeddedClientBuilder::default().verbose(self.verbose);
        if self.asm {
            builder = builder.assembly();
        }
        if let Some(pk) = &self.proving_key {
            builder = builder.proving_key(pk.clone());
        }
        if let Some(pk) = &self.proving_key_plonk {
            builder = builder.proving_key_plonk(pk.clone());
        }
        // `--unlock-mapped-memory` requires `--asm` (clap-enforced), so the
        // Assembly executor is set above and `asm_options` won't panic at build.
        if self.unlock_mapped_memory {
            builder = builder.asm_options(AsmOptions::default().unlock_mapped_memory());
        }
        let client = builder.build()?;

        let mut setup = client.setup(&program);
        if self.hints {
            setup = setup.with_hints();
        }

        setup.run_sync()?;

        info!("{}", "--- SETUP SUMMARY -------------".bright_green().bold());
        info!("Setup completed for {}", elf.display());
        info!("Program name: {}", program.name());
        info!("Hash ID: {}", program.program_id().get_hash());

        Ok(())
    }
}
