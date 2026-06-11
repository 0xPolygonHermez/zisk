use anyhow::Result;

use super::{BuildCmd, NewCmd, RunCmd, ToolchainCmd, UtilsCmd, VerifyCmd};

/// Commands shared between the `cargo-zisk` and `cargo-zisk-dev` CLIs.
///
/// Flattened into each top-level command enum via `#[command(flatten)]`, so the
/// variants appear as first-class subcommands on both binaries.
#[derive(clap::Subcommand)]
pub(crate) enum SharedCmd {
    New(NewCmd),
    Build(BuildCmd),
    Run(RunCmd),
    Verify(VerifyCmd),
    Utils(UtilsCmd),
    Toolchain(ToolchainCmd),
}

impl SharedCmd {
    pub(crate) fn run(self) -> Result<()> {
        match self {
            SharedCmd::New(cmd) => cmd.run(),
            SharedCmd::Build(cmd) => cmd.run(),
            SharedCmd::Run(cmd) => cmd.run(),
            SharedCmd::Verify(cmd) => cmd.run(),
            SharedCmd::Utils(mut cmd) => cmd.run(),
            SharedCmd::Toolchain(mut cmd) => cmd.run(),
        }
    }
}
