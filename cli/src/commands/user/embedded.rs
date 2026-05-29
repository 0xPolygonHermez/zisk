use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

mod execute;
mod prove;
mod wrap;

pub use execute::ZiskEmbeddedExecute;
pub use prove::ZiskEmbeddedProve;
pub use wrap::ZiskEmbeddedWrap;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Run ZisK operations locally using the embedded prover
pub struct ZiskEmbedded {
    #[command(subcommand)]
    pub command: ZiskEmbeddedCommand,
}

#[derive(clap::Subcommand)]
pub enum ZiskEmbeddedCommand {
    /// Generate a proof locally
    Prove(ZiskEmbeddedProve),
    /// Execute a guest program locally
    Execute(ZiskEmbeddedExecute),
    /// Wrap a proof locally
    Wrap(ZiskEmbeddedWrap),
}

impl ZiskEmbedded {
    pub fn run(&mut self) -> Result<()> {
        match &mut self.command {
            ZiskEmbeddedCommand::Prove(cmd) => cmd.run(),
            ZiskEmbeddedCommand::Execute(cmd) => cmd.run(),
            ZiskEmbeddedCommand::Wrap(cmd) => cmd.run(),
        }
    }
}
