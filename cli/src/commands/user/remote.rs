use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

mod execute;
mod prove;
mod setup;
mod upload;
mod wrap;

pub use execute::ZiskRemoteExecute;
pub use prove::ZiskRemoteProve;
pub use setup::ZiskRemoteSetup;
pub use upload::ZiskRemoteUpload;
pub use wrap::ZiskRemoteWrap;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Run ZisK operations against a remote prover service
pub struct ZiskRemote {
    #[command(subcommand)]
    pub command: ZiskRemoteCommand,
}

#[derive(clap::Subcommand)]
pub enum ZiskRemoteCommand {
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

impl ZiskRemote {
    pub fn run(&mut self) -> Result<()> {
        match &mut self.command {
            ZiskRemoteCommand::Upload(cmd) => cmd.run(),
            ZiskRemoteCommand::Setup(cmd) => cmd.run(),
            ZiskRemoteCommand::Prove(cmd) => cmd.run(),
            ZiskRemoteCommand::Execute(cmd) => cmd.run(),
            ZiskRemoteCommand::Wrap(cmd) => cmd.run(),
        }
    }
}
