use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};
use zisk_build::{HELPER_TARGET_SUBDIR, ZISK_TARGET, ZISK_VERSION_MESSAGE};

// Structure representing the 'build' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Build the program to a RISC-V ELF file using the ZisK toolchain
pub struct BuildCmd {
    /// Space or comma separated list of features to activate
    #[arg(short = 'F', long)]
    features: Option<String>,

    /// Activate all available features
    #[arg(long)]
    all_features: bool,

    /// Build artifacts in release mode, with optimizations
    #[arg(long)]
    release: bool,

    /// Do not activate the `default` feature
    #[arg(long)]
    no_default_features: bool,

    /// Copy final artifacts to this directory
    #[arg(long)]
    artifact_dir: Option<String>,

    /// Build only the specified binary (repeat for multiple)
    #[arg(long = "bin", value_name = "BIN")]
    binaries: Vec<String>,

    /// Build only the specified package (repeat for multiple)
    #[arg(short = 'p', long = "package", value_name = "PACKAGE")]
    packages: Vec<String>,

    /// Toolchain name to use
    #[arg(long, hide = true)]
    toolchain_name: Option<String>,
}

impl BuildCmd {
    pub fn run(&self) -> Result<()> {
        // Construct the cargo run command
        let toolchain_name = if let Some(name) = self.toolchain_name.as_deref() {
            println!("Using toolchain_name: {name}");
            name
        } else {
            "zisk"
        };
        let mut command = Command::new("cargo");
        command.args([&format!("+{toolchain_name}"), "build"]);

        // Set RUSTFLAGS for target-cpu=zisk, preserving existing flags
        if let Ok(flags) = std::env::var("RUSTFLAGS") {
            let trimmed = flags.trim();
            if !trimmed.is_empty() {
                command.env("RUSTFLAGS", trimmed);
            }
        }

        command.args(["--target-dir", &format!("target/{}", HELPER_TARGET_SUBDIR)]);

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
        if let Some(artifact_dir) = &self.artifact_dir {
            command.arg("--artifact-dir").arg(artifact_dir);
        }
        for package in &self.packages {
            command.args(["--package", package]);
        }
        for bin in &self.binaries {
            command.args(["--bin", bin]);
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
