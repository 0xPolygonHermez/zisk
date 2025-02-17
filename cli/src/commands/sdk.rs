use clap::Subcommand;

use crate::toolchain::{
    build_toolchain::BuildToolchainCmd, install_toolchain::InstallToolchainCmd, new::NewCmd,
};
use crate::ZISK_VERSION_MESSAGE;
use anyhow::Result;

// Structure representing the 'sdk' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, args_conflicts_with_subcommands = true, version = ZISK_VERSION_MESSAGE)]
pub struct ZiskSdk {
    #[clap(subcommand)]
    pub command: ZiskSdkCommands,
}

// Enum defining the available subcommands for `ZiskSdk`.
#[derive(Subcommand)]
pub enum ZiskSdkCommands {
    BuildToolchain(BuildToolchainCmd),
    InstallToolchain(InstallToolchainCmd),
    New(NewCmd),
}

impl ZiskSdkCommands {
    pub fn run(&self) -> Result<()> {
        match self {
            ZiskSdkCommands::BuildToolchain(cmd) => cmd.run(),
            ZiskSdkCommands::InstallToolchain(cmd) => cmd.run(),
            ZiskSdkCommands::New(cmd) => cmd.run(),
        }
    }
}
