use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, GuestProgram, RemoteClient};

use crate::commands::user::recurser_common::resolve_recurser;
use crate::common::{resolve_elf, ElfSelectorArgs, Profile};
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Upload a guest program ELF or an aggregation program to the remote service
pub(crate) struct ZiskRemoteUpload {
    /// Path to the guest ELF file to upload. If omitted (and no `--aggregation`),
    /// the ELF is auto-detected from the current project
    #[arg(short = 'e', long, conflicts_with = "aggregation")]
    elf: Option<PathBuf>,

    /// Aggregation definition (`<programs>/aggregations/<name>.toml`) to
    /// upload/register as a recurser instead of a guest ELF. Idempotent —
    /// re-uploads of the same definition resolve to the same
    /// content-addressed recurser ID.
    #[arg(short = 'a', long, conflicts_with = "bin")]
    aggregation: Option<PathBuf>,

    #[command(flatten)]
    selector: ElfSelectorArgs,
}

impl ZiskRemoteUpload {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        if let Some(aggregation) = self.aggregation.take() {
            print_banner();
            print_banner_command(format!("{} Upload", "REMOTE".bold()));
            print_banner_field("Aggregation", aggregation.display());
            println!();

            setup_logger(zisk_sdk::VerboseMode::Info);

            let agg = resolve_recurser(&aggregation, self.selector.profile() == Profile::Release)?;
            let result = client.upload(&agg).run()?;

            info!("{}", "--- UPLOAD SUMMARY ------------".bright_green().bold());
            info!("Recurser registered. Recurser ID: {}", result.hash_id());

            return Ok(());
        }

        let elf = resolve_elf(self.elf.take(), self.selector.profile(), self.selector.bin())?;

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
