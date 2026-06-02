use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{GuestProgram, RemoteClient};

use crate::common::resolve_elf;
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate the proving setup for a guest program on the remote service
pub(crate) struct ZiskRemoteSetup {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,

    /// Enable precompiles hints support for this program
    #[arg(long)]
    with_hints: bool,

    /// Generate setup for emulator only (supports `execute`, not `prove`)
    #[arg(long)]
    emulator_only: bool,
}

impl ZiskRemoteSetup {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        let elf = resolve_elf(self.elf.take())?;

        print_banner();
        print_banner_command("Remote Setup");
        print_banner_field("Elf", elf.display());

        let program = GuestProgram::from_uri(elf.to_str().unwrap())?;

        // Register the program before setup — the coordinator needs the ELF bytes.
        let upload = client.upload(&program).run()?;
        info!("Program registered. hash_id: {}", upload.hash_id());

        let mut setup = client.setup(&program);
        if self.with_hints {
            setup = setup.with_hints();
        }
        if self.emulator_only {
            setup = setup.emulator_only();
        }
        setup.run()?.await?;

        info!("{}", "--- SETUP SUMMARY -------------".bright_green().bold());
        info!("Setup completed for hash_id: {}", upload.hash_id());

        Ok(())
    }
}
