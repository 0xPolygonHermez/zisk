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
    New(NewCmd),
    Build(BuildCmd),
    Run(RunCmd),
    Verify(VerifyCmd),
    Utils(UtilsCmd),
    Toolchain(ToolchainCmd),
    Remote(RemoteCmd),
    Embedded(EmbeddedCmd),
}

impl ZiskCliCmd {
    pub(crate) fn run(self) -> Result<()> {
        match self {
            ZiskCliCmd::Build(cmd) => cmd.run(),
            ZiskCliCmd::New(cmd) => cmd.run(),
            ZiskCliCmd::Run(cmd) => cmd.run(),
            ZiskCliCmd::Utils(mut cmd) => cmd.run(),
            ZiskCliCmd::Verify(cmd) => cmd.run(),
            ZiskCliCmd::Toolchain(mut cmd) => cmd.run(),
            ZiskCliCmd::Remote(mut cmd) => cmd.run(),
            ZiskCliCmd::Embedded(mut cmd) => cmd.run(),
        }
    }
}
