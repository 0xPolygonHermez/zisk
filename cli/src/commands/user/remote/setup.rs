use std::path::PathBuf;

use anyhow::{bail, Result};
use zisk_build::ZISK_VERSION_MESSAGE;

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate the proving setup for a guest program on the remote service
pub(crate) struct ZiskRemoteSetup {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,
}

impl ZiskRemoteSetup {
    pub(crate) fn run(&mut self) -> Result<()> {
        print_banner();
        print_banner_command("Remote Setup");

        bail!("`cargo-zisk remote setup` is not implemented yet");
    }
}
