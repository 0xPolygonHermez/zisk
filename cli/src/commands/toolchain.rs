use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

mod build;
mod install;

pub use build::ZiskBuildToolchain;
pub use install::ZiskInstallToolchain;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Toolchain management commands
pub struct ZiskToolchain {
    #[command(subcommand)]
    pub command: ZiskToolchainCommand,
}

#[derive(clap::Subcommand)]
pub enum ZiskToolchainCommand {
    /// Build the cargo-zisk toolchain
    Build(ZiskBuildToolchain),
    /// Install the cargo-zisk toolchain
    Install(ZiskInstallToolchain),
}

impl ZiskToolchain {
    pub fn run(&mut self) -> Result<()> {
        match &mut self.command {
            ZiskToolchainCommand::Build(cmd) => cmd.run(),
            ZiskToolchainCommand::Install(cmd) => cmd.run(),
        }
    }
}
