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
    /// cargo-zisk remote commands for interacting with a remote prover service
    Remote(RemoteCmd),

    /// cargo-zisk embedded commands for running the embedded prover locally
    Embedded(EmbeddedCmd),
    // if no subcommand is provided, default to the embedded prover
    #[command(flatten)]
    EmbeddedDefault(ZiskEmbeddedCmd),
}

impl ZiskCliCmd {
    pub(crate) fn run(self) -> Result<()> {
        match self {
            ZiskCliCmd::Shared(cmd) => cmd.run(),
            ZiskCliCmd::Remote(mut cmd) => cmd.run(),
            ZiskCliCmd::Embedded(mut cmd) => cmd.run(),
            ZiskCliCmd::EmbeddedDefault(mut cmd) => cmd.run(),
        }
    }
}
