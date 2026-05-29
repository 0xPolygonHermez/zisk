use std::path::PathBuf;

use anyhow::{bail, Result};
use zisk_build::ZISK_VERSION_MESSAGE;

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a proof for a guest program on the remote service
pub struct ZiskRemoteProve {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    pub elf: Option<PathBuf>,

    /// Input file path for the guest. Accepts a string literal or a path to a binary file
    #[arg(short = 'i', long)]
    pub inputs: Option<String>,
}

impl ZiskRemoteProve {
    pub fn run(&mut self) -> Result<()> {
        print_banner();
        print_banner_command("Remote Prove");

        bail!("`cargo-zisk remote prove` is not implemented yet");
    }
}
