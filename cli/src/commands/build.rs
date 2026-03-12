use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};
use zisk_build::{ZISK_TARGET, ZISK_VERSION_MESSAGE};

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

    #[arg(short, long)]
    name: Option<String>,

    #[clap(short = 'z', long)]
    zisk_path: Option<String>,

    #[clap(long)]
    hints: bool,
}

impl ZiskBuild {
    pub fn run(&self) -> Result<()> {
        // Construct the cargo run command
        let toolchain_name = if let Some(name) = self.name.as_deref() {
            println!("using toolchain_name: {name}");
            name
        } else {
            "zisk"
        };
        let mut command = Command::new("cargo");
        command.args([&format!("+{toolchain_name}"), "build"]);

        // Set RUSTFLAGS for target-cpu=zisk, preserving existing flags
        let flags = std::env::var("RUSTFLAGS").unwrap_or_default();
        command.env("RUSTFLAGS", flags.trim());

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
        let status = command.status().context("Failed to execute cargo build command")?;
        if !status.success() {
            return Err(anyhow!("Cargo run command failed with status {}", status));
        }

        Ok(())
    }
}
