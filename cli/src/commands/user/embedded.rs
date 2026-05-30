use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

mod execute;
mod prove;
mod wrap;

pub(crate) use execute::ZiskEmbeddedExecute;
pub(crate) use prove::ZiskEmbeddedProve;
pub(crate) use wrap::ZiskEmbeddedWrap;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Run ZisK operations locally using the embedded prover
pub(crate) struct EmbeddedCmd {
    #[command(subcommand)]
    command: ZiskEmbeddedCommand,
}

#[derive(clap::Subcommand)]
pub(crate) enum ZiskEmbeddedCommand {
    /// Generate a proof locally
    Prove(ZiskEmbeddedProve),
    /// Execute a guest program locally
    Execute(ZiskEmbeddedExecute),
    /// Wrap a proof locally
    Wrap(ZiskEmbeddedWrap),
}

impl EmbeddedCmd {
    pub(crate) fn run(&mut self) -> Result<()> {
        match &mut self.command {
            ZiskEmbeddedCommand::Prove(cmd) => cmd.run(),
            ZiskEmbeddedCommand::Execute(cmd) => cmd.run(),
            ZiskEmbeddedCommand::Wrap(cmd) => cmd.run(),
        }
    }
}
