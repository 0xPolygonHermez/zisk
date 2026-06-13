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
/// Generate the proving setup for a guest or aggregation program on the remote service
pub(crate) struct ZiskRemoteSetup {
    /// Path to the guest ELF file. Uploads the program, then runs setup.
    /// If omitted (and no `--hash-id`/`--aggregation`), the ELF is auto-detected
    /// from the current project
    #[arg(short = 'e', long, conflicts_with_all = ["hash_id", "aggregation"])]
    elf: Option<PathBuf>,

    /// Aggregation definition (`<programs>/aggregations/<name>.toml`) to set up
    /// as a recurser instead of a guest ELF. Uploads the recurser spec
    /// (idempotent), then dispatches the setup job. The local vadcop_final
    /// verkey must match the workers' (same setup version) or the derived
    /// recurser_id diverges.
    #[arg(short = 'a', long, conflicts_with_all = ["hash_id", "bin", "hints", "emulator_only"])]
    aggregation: Option<PathBuf>,

    #[command(flatten)]
    selector: ElfSelectorArgs,

    /// hash_id of an already-uploaded program. Runs setup only, skipping the upload.
    #[arg(long)]
    hash_id: Option<String>,

    /// Enable precompiles hints support for this program
    #[arg(long)]
    hints: bool,

    /// Generate setup for emulator only (supports `execute`, not `prove`)
    #[arg(long)]
    emulator_only: bool,
}

impl ZiskRemoteSetup {
    pub(crate) async fn run(&mut self, client: &RemoteClient) -> Result<()> {
        print_banner();
        print_banner_command(format!("{} Setup", "REMOTE".bold()));
        setup_logger(zisk_sdk::VerboseMode::Info);

        if let Some(aggregation) = self.aggregation.take() {
            print_banner_field("Aggregation", aggregation.display());
            println!();

            let agg = resolve_recurser(&aggregation, self.selector.profile() == Profile::Release)?;
            info!("Recurser ID: {}", agg.recurser_id());

            client.upload(&agg).run()?;
            client.setup(&agg).run()?.await?;

            info!("{}", "--- SETUP SUMMARY -------------".bright_green().bold());
            info!("Setup completed for recurser ID: {}", agg.recurser_id());
            return Ok(());
        }

        // `--hash-id`: the program is already uploaded, so just run setup. Otherwise
        // resolve the ELF, upload it (the coordinator needs the bytes), then set up.
        let (hash_id, handle) = if let Some(hash_id) = self.hash_id.take() {
            print_banner_field("Hash ID", &hash_id);
            println!();

            let mut setup = client.setup_by_id(&hash_id);
            if self.hints {
                setup = setup.with_hints();
            }
            if self.emulator_only {
                setup = setup.emulator_only();
            }
            (hash_id, setup.run()?)
        } else {
            let elf = resolve_elf(self.elf.take(), self.selector.profile(), self.selector.bin())?;
            print_banner_field("Elf", elf.display());
            println!();

            let program = GuestProgram::from_uri(elf.to_str().unwrap())?;
            let upload = client.upload(&program).run()?;
            info!("Program registered. Hash ID: {}", upload.hash_id());

            let mut setup = client.setup(&program);
            if self.hints {
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
