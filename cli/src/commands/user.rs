//! `cargo-zisk` CLI commands for building and proving guest programs.

use anyhow::Result;
use clap::Parser;
use zisk_build::ZISK_VERSION_MESSAGE;

mod build;
mod embedded;
mod new;
mod recurser_common;
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

    // cargo-zisk embedded commands for proving and setting up guest programs locally
    #[command(flatten)]
    Embedded(ZiskEmbeddedCmd),

    // User-only commands
    /// cargo-zisk remote commands for interacting with a remote prover service
    Remote(RemoteCmd),
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
