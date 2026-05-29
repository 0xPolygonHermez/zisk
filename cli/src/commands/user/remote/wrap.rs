use std::path::PathBuf;

use anyhow::{bail, Result};
use zisk_build::ZISK_VERSION_MESSAGE;

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Wrap a proof on the remote service
pub struct ZiskRemoteWrap {
    /// Path to the proof to wrap
    #[arg(short = 'p', long)]
    pub proof: Option<PathBuf>,
}

impl ZiskRemoteWrap {
    pub fn run(&mut self) -> Result<()> {
        print_banner();
        print_banner_command("Remote Wrap");

        bail!("`cargo-zisk remote wrap` is not implemented yet");
    }
}
