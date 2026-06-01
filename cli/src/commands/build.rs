use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};
use zisk_build::{HELPER_TARGET_SUBDIR, ZISK_TARGET, ZISK_VERSION_MESSAGE};

use crate::common::ensure_zisk_target_installed;

use super::utils::ZISK_LINKER_SCRIPT;

// Structure representing the 'build' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Build the program to a RISC-V ELF file using the ZisK toolchain
pub struct ZiskBuild {
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
}

impl ZiskBuild {
    pub fn run(&self) -> Result<()> {
        // Ensure the Zisk Rust target is installed before invoking cargo
        ensure_zisk_target_installed()?;

        // Construct the cargo build command
        let mut command = Command::new("cargo");
        command.arg("build");

        // Generate the linker script from the embedded bytes and write it to a temporary file
        let linker_script_path = std::env::temp_dir().join("zisk.ld");
        std::fs::write(&linker_script_path, ZISK_LINKER_SCRIPT)
            .context("Failed to write Zisk linker script to temp dir")?;

        // Add linker script flag and zisk_guest cfg to RUSTFLAGS, preserving any existing flags
        let current_rust_flags = std::env::var("RUSTFLAGS").unwrap_or_default();
        let rust_flags = format!(
            "{} --cfg zisk_guest -C link-arg=-T{}",
            current_rust_flags.trim(),
            linker_script_path.display()
        )
        .trim()
        .to_string();

        command.env("RUSTFLAGS", rust_flags);

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

        // Debug: print the cargo invocation (env overrides + program + args)
        let env_overrides = command
            .get_envs()
            .filter_map(|(k, v)| v.map(|v| format!("{}={:?}", k.to_string_lossy(), v.to_string_lossy())))
            .collect::<Vec<_>>()
            .join(" ");
        let args = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        println!(
            "Running: {} {} {}",
            env_overrides,
            command.get_program().to_string_lossy(),
            args,
        );

        // Execute the command
        let status = command.status().context("Failed to execute cargo build command")?;
        if !status.success() {
            return Err(anyhow!("Cargo run command failed with status {}", status));
        }

        Ok(())
    }
}
