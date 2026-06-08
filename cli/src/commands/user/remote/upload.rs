use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, GuestProgram, RemoteClient};

use crate::common::{resolve_elf, ProfileArgs};
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Upload a guest program ELF to the remote service
pub(crate) struct ZiskRemoteUpload {
    /// Path to the guest ELF file to upload. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,

    #[command(flatten)]
    profile: ProfileArgs,
}

impl ZiskRemoteUpload {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        let elf = resolve_elf(self.elf.take(), self.profile.profile())?;

        print_banner();
        print_banner_command(format!("{} Upload", "REMOTE".bold()));
        print_banner_field("Elf", elf.display());
        println!();

        setup_logger(zisk_sdk::VerboseMode::Info);

        let program = GuestProgram::from_uri(elf.to_str().unwrap())?;
        let result = client.upload(&program).run()?;

        info!("{}", "--- UPLOAD SUMMARY ------------".bright_green().bold());
        info!("Program registered. Hash ID: {}", result.hash_id());

        Ok(())
    }
}
