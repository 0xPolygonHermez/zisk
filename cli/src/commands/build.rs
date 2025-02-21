use crate::{ZISK_TARGET, ZISK_VERSION_MESSAGE};
use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};

// Structure representing the 'build' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
pub struct ZiskBuild {
    #[clap(short = 'F', long)]
    features: Option<String>,

    #[clap(long)]
    all_features: bool,

    #[clap(long)]
    release: bool,

    #[clap(long)]
    no_default_features: bool,
}

impl ZiskBuild {
    pub fn run(&self) -> Result<()> {
        // Construct the cargo run command
        let mut command = Command::new("cargo");
        command.args(["+zisk", "build"]);
        // Add the feature selection flags
        if let Some(features) = &self.features {
            command.arg("--features").arg(features);
        }
        if self.all_features {
            command.arg("--all-features");
        }
        if self.no_default_features {
            command.arg("--no-default-features");
        }
        if self.release {
            command.arg("--release");
        }

        command.args(["--target", ZISK_TARGET]);

        // Set up the command to inherit the parent's stdout and stderr
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        // Execute the command
        let status = command.status().context("Failed to execute cargo run command")?;
        if !status.success() {
            return Err(anyhow!("Cargo run command failed with status {}", status));
        }

        Ok(())
    }
}
