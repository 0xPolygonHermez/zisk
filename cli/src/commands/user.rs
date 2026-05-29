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

#[derive(Parser)]
#[command(
    name = "cargo-zisk",
    bin_name = "cargo-zisk",
    version = ZISK_VERSION_MESSAGE,
    about = "CLI for ZisK for building and proving guest programs",
    long_about = "Cargo Zisk is the CLI for ZisK for building and proving guest programs."
)]
pub enum ZiskCmd {
    New(ZiskNew),
    Build(ZiskBuild),
    Run(ZiskRun),
    Verify(ZiskVerify),
    Utils(ZiskUtils),
    Toolchain(ZiskToolchain),
    Remote(ZiskRemote),
    Embedded(ZiskEmbedded),
}

impl ZiskCmd {
    pub fn run(self) -> Result<()> {
        match self {
            ZiskCmd::Build(cmd) => cmd.run(),
            ZiskCmd::New(cmd) => cmd.run(),
            ZiskCmd::Run(cmd) => cmd.run(),
            ZiskCmd::Utils(mut cmd) => cmd.run(),
            ZiskCmd::Verify(cmd) => cmd.run(),
            ZiskCmd::Toolchain(mut cmd) => cmd.run(),
            ZiskCmd::Remote(mut cmd) => cmd.run(),
            ZiskCmd::Embedded(mut cmd) => cmd.run(),
        }
    }
}
