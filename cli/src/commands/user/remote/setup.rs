use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_sdk::{setup_logger, GuestProgram, RemoteClient};

use crate::common::resolve_elf;
use crate::ux::{print_banner, print_banner_command, print_banner_field};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate the proving setup for a guest program on the remote service
pub(crate) struct ZiskRemoteSetup {
    /// Path to the guest ELF file. Uploads the program, then runs setup.
    /// If omitted (and no `--hash-id`), the ELF is auto-detected from the current project
    #[arg(short = 'e', long, conflicts_with = "hash_id")]
    elf: Option<PathBuf>,

    /// hash_id of an already-uploaded program. Runs setup only, skipping the upload.
    #[arg(long, conflicts_with = "elf")]
    hash_id: Option<String>,

    /// Enable precompiles hints support for this program
    #[arg(long)]
    with_hints: bool,

    /// Generate setup for emulator only (supports `execute`, not `prove`)
    #[arg(long)]
    emulator_only: bool,
}

impl ZiskRemoteSetup {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        print_banner();
        print_banner_command(format!("{} Setup", "REMOTE".bold()));
        setup_logger(zisk_sdk::VerboseMode::Info);

        // `--hash-id`: the program is already uploaded, so just run setup. Otherwise
        // resolve the ELF, upload it (the coordinator needs the bytes), then set up.
        let (hash_id, handle) = if let Some(hash_id) = self.hash_id.take() {
            print_banner_field("Hash ID", &hash_id);
            println!();

            let mut setup = client.setup_by_id(&hash_id);
            if self.with_hints {
                setup = setup.with_hints();
            }
            if self.emulator_only {
                setup = setup.emulator_only();
            }
            (hash_id, setup.run()?)
        } else {
            let elf = resolve_elf(self.elf.take())?;
            print_banner_field("Elf", elf.display());
            println!();

            let program = GuestProgram::from_uri(elf.to_str().unwrap())?;
            let upload = client.upload(&program).run()?;
            info!("Program registered. Hash ID: {}", upload.hash_id());

            let mut setup = client.setup(&program);
            if self.with_hints {
                setup = setup.with_hints();
            }
            if self.emulator_only {
                setup = setup.emulator_only();
            }
            (upload.hash_id().to_string(), setup.run()?)
        };
        handle.await?;

        info!("{}", "--- SETUP SUMMARY -------------".bright_green().bold());
        info!("Setup completed for Hash ID: {hash_id}");

        Ok(())
    }
}
