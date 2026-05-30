use std::path::PathBuf;

use anyhow::{bail, Result};
use zisk_build::ZISK_VERSION_MESSAGE;

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate a proof for a guest program locally
pub(crate) struct ZiskEmbeddedProve {
    /// Path to the guest ELF file. If omitted, the ELF is auto-detected from the current project
    #[arg(short = 'e', long)]
    elf: Option<PathBuf>,

    /// Input file path for the guest. Accepts a string literal or a path to a binary file
    #[arg(short = 'i', long)]
    inputs: Option<String>,
}

impl ZiskEmbeddedProve {
    pub(crate) fn run(&mut self) -> Result<()> {
        print_banner();
        print_banner_command("Embedded Prove");

        bail!("`cargo-zisk embedded prove` is not implemented yet");
    }
}
