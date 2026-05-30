use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

mod execute;
mod prove;
mod setup;
mod upload;
mod wrap;

pub(crate) use execute::ZiskRemoteExecute;
pub(crate) use prove::ZiskRemoteProve;
pub(crate) use setup::ZiskRemoteSetup;
pub(crate) use upload::ZiskRemoteUpload;
pub(crate) use wrap::ZiskRemoteWrap;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Run ZisK operations against a remote prover service
pub(crate) struct RemoteCmd {
    #[command(subcommand)]
    command: ZiskRemoteCommand,
}

#[derive(clap::Subcommand)]
pub(crate) enum ZiskRemoteCommand {
    /// Upload a guest program to the remote service
    Upload(ZiskRemoteUpload),
    /// Generate the proving setup on the remote service
    Setup(ZiskRemoteSetup),
    /// Generate a proof on the remote service
    Prove(ZiskRemoteProve),
    /// Execute a guest program on the remote service
    Execute(ZiskRemoteExecute),
    /// Wrap a proof on the remote service
    Wrap(ZiskRemoteWrap),
}

impl RemoteCmd {
    pub(crate) fn run(&mut self) -> Result<()> {
        match &mut self.command {
            ZiskRemoteCommand::Upload(cmd) => cmd.run(),
            ZiskRemoteCommand::Setup(cmd) => cmd.run(),
            ZiskRemoteCommand::Prove(cmd) => cmd.run(),
            ZiskRemoteCommand::Execute(cmd) => cmd.run(),
            ZiskRemoteCommand::Wrap(cmd) => cmd.run(),
        }
    }
}
