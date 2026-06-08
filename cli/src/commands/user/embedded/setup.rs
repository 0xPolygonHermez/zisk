use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, EmbeddedClientBuilder, GuestProgram, VerboseMode};

use super::validate_setup_asm;
use crate::common::resolve_elf;
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate the proving setup for a guest program locally
pub(crate) struct ZiskEmbeddedSetup {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,

    /// Use the ASM emulator instead of the default Rust emulator
    #[arg(short = 'a', long)]
    asm: bool,

    /// Enable precompiles hints support for this program. Requires the ASM backend (`--asm`).
    #[arg(long, requires = "asm", conflicts_with = "emulator_only")]
    with_hints: bool,

    /// Verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl ZiskEmbeddedSetup {
    pub(crate) fn run(&mut self) -> Result<()> {
        let elf = resolve_elf(self.elf.take())?;
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
        let client = builder.build()?;

        let mut setup = client.setup(&program);
        if self.with_hints {
            setup = setup.with_hints();
        }

        setup = setup.emulator_only();

        setup.run_sync()?;

        info!("{}", "--- SETUP SUMMARY -------------".bright_green().bold());
        info!("Setup completed for {}", elf.display());
        info!("Program name: {}", program.name());
        info!("Hash ID: {}", program.program_id().get_hash());

        Ok(())
    }
}
