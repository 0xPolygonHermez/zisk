use std::path::PathBuf;

use anyhow::{bail, Result};
use zisk_build::ZISK_VERSION_MESSAGE;

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Upload a guest program ELF to the remote service
pub(crate) struct ZiskRemoteUpload {
    /// Path to the guest ELF file to upload. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,
}

impl ZiskRemoteUpload {
    pub(crate) fn run(&mut self) -> Result<()> {
        print_banner();
        print_banner_command("Remote Upload");

        bail!("`cargo-zisk remote upload` is not implemented yet");
    }
}
