mod dev;
mod shared;
mod user;

pub(crate) use dev::*;
pub(crate) use shared::*;
pub(crate) use user::*;

use anyhow::Result;
use clap::Parser;

/// Parses developer CLI arguments and dispatches to the selected command.
pub fn run_cli_dev() -> Result<()> {
    ZiskCliDevCmd::parse().run()
}

/// Parses the user-facing CLI arguments and dispatches to the selected command.
pub fn run_cli() -> Result<()> {
    ZiskCliCmd::parse().run()
}
