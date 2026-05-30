use std::path::PathBuf;

use anyhow::{bail, Result};
use zisk_build::ZISK_VERSION_MESSAGE;

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Wrap a proof locally
pub(crate) struct ZiskEmbeddedWrap {
    /// Path to the proof to wrap
    #[arg(short = 'p', long)]
    proof: Option<PathBuf>,
}

impl ZiskEmbeddedWrap {
    pub(crate) fn run(&mut self) -> Result<()> {
        print_banner();
        print_banner_command("Embedded Wrap");

        bail!("`cargo-zisk embedded wrap` is not implemented yet");
    }
}
