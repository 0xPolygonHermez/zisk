use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use zisk_build::ZISK_VERSION_MESSAGE;

#[derive(clap::Args, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Run cargo clean in the current project
pub(crate) struct CleanCmd;

impl CleanCmd {
    pub(crate) fn run(&self) -> Result<()> {
        let status = Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Failed to execute cargo clean command")?;

        if !status.success() {
            anyhow::bail!("Cargo clean command failed with status {}", status);
        }

        Ok(())
    }
}
