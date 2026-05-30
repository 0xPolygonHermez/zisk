use anyhow::Result;
use clap::Parser;
use zisk_build::ZISK_VERSION_MESSAGE;

mod build;
mod embedded;
mod new;
mod remote;
mod run;
mod toolchain;
mod utils;
mod verify;

pub(crate) use build::*;
pub(crate) use embedded::*;
pub(crate) use new::*;
pub(crate) use remote::*;
pub(crate) use run::*;
pub(crate) use toolchain::*;
pub(crate) use utils::*;
pub(crate) use verify::*;

use super::SharedCmd;

/// Parses the user-facing CLI arguments and dispatches to the selected command.
pub fn run_cli() -> Result<()> {
    ZiskCliCmd::parse().run()
}

#[derive(Parser)]
#[command(
    name = "cargo-zisk",
    bin_name = "cargo-zisk",
    version = ZISK_VERSION_MESSAGE,
    about = "CLI for ZisK for building and proving guest programs",
    long_about = "Cargo Zisk is the CLI for ZisK for building and proving guest programs."
)]
pub(crate) enum ZiskCliCmd {
    // Commands shared with cargo-zisk-dev
    #[command(flatten)]
    Shared(SharedCmd),

    // User-only commands
    Remote(RemoteCmd),
    Embedded(EmbeddedCmd),
}

impl ZiskCliCmd {
    pub(crate) fn run(self) -> Result<()> {
        match self {
            ZiskCliCmd::Shared(cmd) => cmd.run(),
            ZiskCliCmd::Remote(mut cmd) => cmd.run(),
            ZiskCliCmd::Embedded(mut cmd) => cmd.run(),
        }
    }
}
